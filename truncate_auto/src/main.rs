use tokio::sync::mpsc::error::SendError;
use truncate_auto::service::{self, tile, SquareValidity};
use truncate_auto::{invert_move, move_request_to_move, move_to_player_move};

use service::play_game_request;
use service::truncate_server::{Truncate, TruncateServer};
use service::{ErrorReply, MoveReply, PlayGameReply, PlayGameRequest};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::{error::Error, io::ErrorKind, net::ToSocketAddrs, pin::Pin};
use tokio::sync::{mpsc, oneshot};
use tokio_stream::{wrappers::ReceiverStream, Stream, StreamExt};
use tonic::{transport::Server, Response, Status};
use truncate_core::board::{Board, Square};
use truncate_core::player::Hand;
use truncate_core::rules::{BoardOrientation, GameRules};
use truncate_core::{game::Game, judge::WordData, moves::Move};

pub struct AutoServer {
    pending_game: Arc<Mutex<Option<oneshot::Sender<GamePlayer>>>>,
    valid_words: Arc<HashMap<String, WordData>>,
}

type ResponseStream = Pin<Box<dyn Stream<Item = Result<PlayGameReply, Status>> + Send>>;

fn error_reply(request_id: impl Into<String>, msg: impl Into<String>) -> PlayGameReply {
    return PlayGameReply {
        request_id: request_id.into(),
        reply: Some(service::play_game_reply::Reply::ErrorReply(ErrorReply {
            error: msg.into(),
        })),
    };
}

fn to_hand(h: &Hand) -> Vec<String> {
    h.0.iter().map(|c| c.to_string()).collect()
}

fn square_to_board_tile(sq: &Square) -> service::Tile {
    service::Tile {
        tile: Some(match sq {
            Square::Water { foggy } => tile::Tile::Water(service::WaterTile { foggy: *foggy }),
            Square::Land { foggy } => tile::Tile::Land(service::LandTile { foggy: *foggy }),
            Square::Town {
                player,
                defeated,
                foggy,
            } => tile::Tile::Town(service::TownTile {
                player: *player as u32,
                defeated: *defeated,
                foggy: *foggy,
            }),
            Square::Obelisk { foggy } => {
                tile::Tile::Obelisk(service::ObeliskTile { foggy: *foggy })
            }
            Square::Artifact {
                player,
                defeated,
                foggy,
            } => tile::Tile::Artifact(service::ArtifactTile {
                player: *player as u32,
                defeated: *defeated,
                foggy: *foggy,
            }),
            Square::Occupied {
                player,
                tile,
                validity,
                foggy,
            } => tile::Tile::Occupied(service::OccupiedTile {
                player: *player as u32,
                tile: tile.to_string(),
                validity: match validity {
                    truncate_core::board::SquareValidity::Unknown => {
                        SquareValidity::Unspecified as i32
                    }
                    truncate_core::board::SquareValidity::Valid => SquareValidity::Valid as i32,
                    truncate_core::board::SquareValidity::Invalid => SquareValidity::Invalid as i32,
                    truncate_core::board::SquareValidity::Partial => SquareValidity::Partial as i32,
                },
                foggy: *foggy,
            }),
            Square::Fog {} => tile::Tile::Fog(service::FogTile {}),
        }),
    }
}

fn to_squares(sqs: &Vec<Square>) -> service::Squares {
    service::Squares {
        tiles: sqs.iter().map(|s| square_to_board_tile(s)).collect(),
    }
}

fn to_board(b: &Board) -> service::Board {
    let mut squares: Vec<service::Squares> = vec![];

    for row in &b.squares {
        squares.push(to_squares(row))
    }

    service::Board { squares }
}

fn to_player_board(g: &Game, player: usize) -> service::Board {
    let (filtered_board, _) = g.filter_game_to_player(player);
    to_board(&filtered_board)
}

#[derive(Debug)]
struct GamePlayer {
    id: usize,
    name: String,
    sender: mpsc::Sender<Result<PlayGameReply, Status>>,
    // Streaming is not Sync, so we wrap it in a mutex
    stream: mpsc::Receiver<Result<PlayGameRequest, Status>>,
    initial_req_id: String,
}

impl GamePlayer {
    async fn send(&self, reply: PlayGameReply) -> Result<(), Status> {
        self.sender.send(Ok(reply)).await.map_err(|e| {
            Status::internal(format!(
                "failed to send message to client {}: {:?}",
                self.id, e
            ))
        })
    }

    async fn get_message(&mut self) -> Result<PlayGameRequest, Status> {
        match self.stream.recv().await {
            Some(r) => match r {
                Ok(rr) => Ok(rr),
                Err(err) => {
                    if let Some(io_err) = match_for_io_error(&err) {
                        if io_err.kind() == ErrorKind::BrokenPipe {
                            return Err(Status::failed_precondition(format!(
                                "player {} dropped",
                                self.id
                            )));
                        }
                    }
                    Err(Status::failed_precondition(format!(
                        "player {} errored {:?}",
                        self.id, err
                    )))
                }
            },
            None => Err(Status::failed_precondition(format!(
                "player {} dropped",
                self.id
            ))),
        }
    }
}

#[derive(Debug)]
struct GameHandler {
    players: Vec<GamePlayer>,
    game: Game,
    valid_words: Arc<HashMap<String, WordData>>,
}

impl GameHandler {
    async fn run_game(&mut self) {
        let status = match self.run_game_internal().await {
            Ok(_) => {
                println!("Ran game successfully!");
                return;
            }
            Err(st) => st,
        };

        // Send the error to all users.
        // TODO: Maybe audit the set of things we might send here, not all will necessarily make sense.
        let mut f = vec![];
        for gp in &self.players {
            f.push(gp.sender.send(Err(status.clone())));
        }
        futures::future::join_all(f).await;
    }

    async fn run_game_internal(&mut self) -> Result<(), Status> {
        // Send down the initial reply to each player
        self.send_init_reply().await?;

        loop {
            // Let the next player know it's their turn to move.
            self.send_move_solicitation().await?;

            // Now wait for them to move.
            let (id, req) = self.get_move_from_player().await?;

            let (player_reply, others_reply) = self.handle_move(id, req)?;
            self.players
                .get(id) // XXX: This relies on player_ids matching their index in `players`
                .unwrap()
                .sender
                .send(Ok(player_reply))
                .await
                .map_err(|e| Status::internal(format!("failed to send reply {:?}", e)))?;

            if let Some((other_move, gr)) = others_reply {
                let mut v = vec![];
                for gp in &self.players {
                    if gp.id != id {
                        // Possible TODO: This does __not__ handle fog of war coordinate mutations.
                        let tailored_reply = PlayGameReply {
                            request_id: "".to_string(),
                            reply: Some(service::play_game_reply::Reply::PlayerMove(
                                move_to_player_move(
                                    to_player_board(&self.game, gp.id),
                                    &invert_move(&self.game, &other_move),
                                    gr,
                                ),
                            )),
                        };
                        v.push(gp.sender.send(Ok(tailored_reply)));
                    }
                }
                futures::future::join_all(v).await;
            }
        }
    }

    fn current_player<'a>(&'a self) -> Option<&'a GamePlayer> {
        let next_player = match self.game.next_player {
            Some(v) => v,
            None => return None,
        };

        self.players.get(next_player)
    }

    fn current_player_mut<'a>(&'a mut self) -> Option<&'a mut GamePlayer> {
        let next_player = match self.game.next_player {
            Some(v) => v,
            None => return None,
        };

        self.players.get_mut(next_player)
    }

    // First move is a response to the player, second is for everyone else.
    fn handle_move(
        &mut self,
        player_id: usize,
        req: PlayGameRequest,
    ) -> Result<(PlayGameReply, Option<(Move, bool)>), Status> {
        // Do the move,
        match self.play_game_request_to_move(player_id, &req) {
            Ok(game_move) => {
                let game_resp = self.game.play_turn(
                    game_move.clone(),
                    Some(&self.valid_words),
                    Some(&self.valid_words),
                    None,
                );
                match game_resp {
                    Ok(gr) => {
                        return Ok((
                            PlayGameReply {
                                request_id: req.request_id.clone(),
                                reply: Some(service::play_game_reply::Reply::MoveReply(
                                    MoveReply {
                                        hand: to_hand(
                                            &self.game.players.get(player_id).unwrap().hand,
                                        ),
                                        board: Some(to_player_board(&self.game, player_id)),
                                        game_over: gr.is_some(),
                                    },
                                )),
                            },
                            Some((game_move, gr.is_some())),
                        ));
                    }
                    Err(msg) => return Ok((error_reply(req.request_id, msg), None)),
                }
            }
            Err(resp) => {
                return Ok((resp, None));
            }
        }
    }

    fn play_game_request_to_move(
        &self,
        player_id: usize,
        req: &PlayGameRequest,
    ) -> Result<Move, PlayGameReply> {
        match &req.request {
            Some(play_game_request::Request::InitRequest(_)) => {
                return Err(error_reply(
                    &req.request_id,
                    "player sent InitRequest after game had begun",
                ))
            }
            Some(play_game_request::Request::MoveRequest(mr)) => {
                match move_request_to_move(player_id, mr) {
                    Some(mv) => Ok(mv),
                    None => {
                        return Err(error_reply(
                            &req.request_id,
                            "No move was given in MoveRequest",
                        ))
                    }
                }
            }
            None => {
                return Err(error_reply(
                    &req.request_id,
                    "PlayGameRequest contained no request",
                ))
            }
        }
    }

    async fn send_init_reply(&self) -> Result<(), Status> {
        let mut v = vec![];
        for gp in &self.players {
            let board = to_player_board(&self.game, gp.id);
            let hand = to_hand(&self.game.players.get(gp.id).unwrap().hand);
            let opponents = self
                .game
                .players
                .iter()
                .enumerate()
                .filter(|(i, _v)| *i != gp.id)
                .map(|(_i, v)| service::Player {
                    id: gp.id as u32,
                    name: v.name.clone(),
                })
                .collect();

            let reply = PlayGameReply {
                request_id: gp.initial_req_id.clone(),
                reply: Some(service::play_game_reply::Reply::InitReply(
                    service::InitReply {
                        player_id: gp.id as u32,
                        hand,
                        board: Some(board),
                        opponents,
                    },
                )),
            };
            v.push(gp.sender.send(Ok(reply)));
        }
        for res in futures::future::join_all(v).await {
            match res {
                Err(SendError(Err(status))) => {
                    return Err(Status::internal(format!(
                        "failed to send message to player: {:?}",
                        status
                    )))
                }
                _ => { /* probably fine */ }
            };
        }
        Ok(())
    }

    async fn send_move_solicitation(&self) -> Result<(), Status> {
        let cur_player = match self.current_player() {
            Some(cp) => cp,
            None => {
                eprintln!("no current player!");
                return Err(Status::internal("couldn't find a current player!"));
            }
        };

        // Let the player know we'd like a move from them.
        let board = to_player_board(&self.game, cur_player.id);
        cur_player
            .send(PlayGameReply {
                request_id: "".to_string(),
                reply: Some(service::play_game_reply::Reply::MoveSolicitation(
                    service::MoveSolicitation { board: Some(board) },
                )),
            })
            .await?;
        Ok(())
    }

    async fn get_move_from_player(&mut self) -> Result<(usize, PlayGameRequest), Status> {
        let cur_player = match self.current_player_mut() {
            Some(cp) => cp,
            None => {
                eprintln!("no current player!");
                return Err(Status::internal("couldn't find a current player!"));
            }
        };
        Ok((cur_player.id, cur_player.get_message().await?))
    }
}

#[tonic::async_trait]
impl Truncate for AutoServer {
    type PlayGameStream = ResponseStream;

    async fn play_game(
        &self,
        req: tonic::Request<tonic::Streaming<PlayGameRequest>>,
    ) -> std::result::Result<tonic::Response<Self::PlayGameStream>, Status> {
        let ((gp_tx, gp_rx), player_id) = {
            let mut pending_game = self
                .pending_game
                .lock()
                .map_err(|e| Status::internal(format!("failed to local pending game ID: {}", e)))?;
            match pending_game.take() {
                Some(tx) => ((Some(tx), None), 1),
                None => {
                    let (tx, rx) = oneshot::channel();
                    *pending_game = Some(tx);
                    ((None, Some(rx)), 0)
                }
            }
        };

        let mut in_stream = req.into_inner();
        let (tx, rx) = mpsc::channel(128);

        // Okay, the game is on! We expect the first message to be an init request.
        let (player_id, request_id, player_name) = match in_stream.next().await {
            Some(Ok(PlayGameRequest {
                request_id,
                request: Some(play_game_request::Request::InitRequest(ir)),
            })) => (player_id, request_id, ir.player_name),
            Some(Ok(_)) => {
                return Err(Status::failed_precondition(
                    "initial message from client was not an init request",
                ))
            }
            Some(Err(e)) => {
                eprintln!("received error status from client {}", e);
                return Err(Status::failed_precondition("client errored"));
            }
            None => {
                eprintln!("stream is over already?");
                return Err(Status::failed_precondition("no move was given?"));
            }
        };

        let (in_tx, in_rx) = mpsc::channel(10);

        tokio::spawn(async move {
            while let Some(msg) = in_stream.next().await {
                if let Err(e) = in_tx.send(msg).await {
                    match e.0 {
                        Ok(v) => eprintln!("failed to forward message: {:?}", v),
                        Err(status) => {
                            if let Some(io_err) = match_for_io_error(&status) {
                                if io_err.kind() == ErrorKind::BrokenPipe {
                                    eprintln!("player {} dropped", player_id);
                                    return;
                                }
                            }

                            eprintln!("error forwarding message: {:?}", status);
                        }
                    }
                }
            }
        });

        let gp = GamePlayer {
            id: player_id,
            name: player_name,
            sender: tx,
            stream: in_rx,
            initial_req_id: request_id,
        };

        let game_handler = match (player_id, gp_tx, gp_rx) {
            (0, None, Some(rx)) => {
                // If we're player zero, wait to get info from player one.
                let other_player_gp = rx.await.map_err(|e| {
                    Status::internal(format!("failed to get other player info: {:?}", e))
                })?;

                let mut game = Game::new(9, 9, None, GameRules::generation(2));
                game.rules.board_orientation = BoardOrientation::FacingPlayer;
                game.add_player(gp.name.clone());
                game.add_player(other_player_gp.name.clone());
                game.rules.battle_delay = 0;
                game.board.cache_special_squares();
                game.start();
                Some(GameHandler {
                    players: vec![gp, other_player_gp],
                    game,
                    valid_words: Arc::clone(&self.valid_words),
                })
            }
            (1, Some(gtx), None) => {
                // If we're player one, send everything to player zero.
                gtx.send(gp).map_err(|e| {
                    Status::internal(format!("failed to send info to player zero: {:?}", e))
                })?;
                None
            }
            _ => {
                return Err(Status::internal(format!(
                    "unexpected player id {}",
                    player_id
                )));
            }
        };

        if let Some(mut gh) = game_handler {
            tokio::spawn(async move {
                gh.run_game().await;
            });
        }

        let out_stream = ReceiverStream::new(rx);

        Ok(Response::new(Box::pin(out_stream) as Self::PlayGameStream))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let valid_words = truncate_auto::init_dict()?;

    let server = AutoServer {
        pending_game: Arc::new(Mutex::new(None)),
        valid_words: Arc::new(valid_words),
    };

    Server::builder()
        .add_service(TruncateServer::new(server))
        .serve("[::1]:50051".to_socket_addrs().unwrap().next().unwrap())
        .await
        .unwrap();

    Ok(())
}

fn match_for_io_error(err_status: &Status) -> Option<&std::io::Error> {
    let mut err: &(dyn Error + 'static) = err_status;

    loop {
        if let Some(io_err) = err.downcast_ref::<std::io::Error>() {
            return Some(io_err);
        }

        // h2::Error do not expose std::io::Error with `source()`
        // https://github.com/hyperium/h2/pull/462
        if let Some(h2_err) = err.downcast_ref::<h2::Error>() {
            if let Some(io_err) = h2_err.get_io() {
                return Some(io_err);
            }
        }

        err = match err.source() {
            Some(err) => err,
            None => return None,
        };
    }
}
