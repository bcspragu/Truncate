use tokio::sync::mpsc::error::SendError;
use tonic::Streaming;
use truncate_auto::service::{self, tile, SquareValidity};
use truncate_auto::{move_request_to_move, move_to_player_move};

use service::play_game_request;
use service::truncate_server::{Truncate, TruncateServer};
use service::{ErrorReply, MoveReply, PlayGameReply, PlayGameRequest};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::{error::Error, io::ErrorKind, net::ToSocketAddrs, pin::Pin};
use tokio::sync::{mpsc, oneshot};
use tokio_stream::{wrappers::ReceiverStream, Stream, StreamExt};
use tonic::{transport::Server, Response, Status};
use truncate_core::board::{Board, Square};
use truncate_core::player::Hand;
use truncate_core::rules::GameRules;
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

#[derive(Debug)]
struct GamePlayer {
    id: usize,
    name: String,
    sender: mpsc::Sender<Result<PlayGameReply, Status>>,
    stream: Streaming<PlayGameRequest>,
    initial_req_id: String,
}

impl GamePlayer {
    async fn get_message(&mut self) -> Result<PlayGameRequest, Status> {
        match self.stream.next().await {
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
    game: Arc<RwLock<Game>>,
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
        let mut f = vec![];
        for gp in &self.players {
            f.push(gp.sender.send(Err(status.clone())));
        }
        futures::future::join_all(f).await;
    }

    async fn run_game_internal(&mut self) -> Result<(), Status> {
        // Send down the initial reply to each player
        let mut v = vec![];
        for gp in &self.players {
            let game = self.game.read().unwrap();

            let board = to_board(&game.board);
            let hand = to_hand(&game.players.get(gp.id).unwrap().hand);
            let opponents = game
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
                reply: Some(service::play_game_reply::Reply::PlayReply(
                    service::PlayReply {
                        player_id: gp.id as u32,
                        hand,
                        board: Some(board),
                        opponents,
                        first_to_move: 0,
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

        loop {
            {
                let cur_player = match self.current_player() {
                    Some(cp) => cp,
                    None => {
                        eprintln!("no current player!");
                        return Err(Status::internal("couldn't find a current player!"));
                    }
                };

                // Let the player know we'd like a move from them.
                let board = to_board(&self.game.read().unwrap().board);
                cur_player
                    .sender
                    .send(Ok(PlayGameReply {
                        request_id: "".to_string(),
                        reply: Some(service::play_game_reply::Reply::MoveSolicitation(
                            service::MoveSolicitation { board: Some(board) },
                        )),
                    }))
                    .await
                    .map_err(|e| {
                        Status::internal(format!("failed to send move solicitation: {:?}", e))
                    })?;
            }

            // Now wait for their move
            let req = {
                let zz = match self.current_player_mut() {
                    Some(cp) => cp,
                    None => {
                        eprintln!("no current player!");
                        return Err(Status::internal("couldn't find a current player!"));
                    }
                };
                zz.get_message().await?.clone()
            };
            let (player_id, others_reply) = {
                let cur_player = match self.current_player() {
                    Some(cp) => cp,
                    None => {
                        eprintln!("no current player!");
                        return Err(Status::internal("couldn't find a current player!"));
                    }
                };

                let (player_reply, others_reply) = self.handle_move(cur_player.id, req)?;
                let id = cur_player.id;
                cur_player
                    .sender
                    .send(Ok(player_reply))
                    .await
                    .map_err(|e| Status::internal(format!("failed to send reply {:?}", e)))?;
                (id, others_reply)
            };

            if let Some(other_reply) = others_reply {
                let mut v = vec![];
                for gp in &self.players {
                    if gp.id != player_id {
                        v.push(gp.sender.send(Ok(other_reply.clone())));
                    }
                }
                futures::future::join_all(v).await;
            }
        }
    }

    fn current_player<'a>(&'a self) -> Option<&'a GamePlayer> {
        let next_player = match self.game.read().unwrap().next_player {
            Some(v) => v,
            None => return None,
        };

        self.players.get(next_player)
    }

    fn current_player_mut<'a>(&'a mut self) -> Option<&'a mut GamePlayer> {
        let next_player = match self.game.read().unwrap().next_player {
            Some(v) => v,
            None => return None,
        };

        self.players.get_mut(next_player)
    }

    // First move is a response to the player, second is for everyone else.
    fn handle_move(
        &self,
        player_id: usize,
        req: PlayGameRequest,
    ) -> Result<(PlayGameReply, Option<PlayGameReply>), Status> {
        // Do the move,
        match self.play_game_request_to_move(player_id, &req) {
            Ok(game_move) => {
                println!("GOT MOVE {:?}", game_move);
                let game_resp = self.game.write().unwrap().play_turn(
                    game_move.clone(),
                    Some(&self.valid_words),
                    Some(&self.valid_words),
                    None,
                );
                match game_resp {
                    Ok(gr) => {
                        let game = self.game.read().unwrap();
                        println!(
                            "player {} has hand {}",
                            player_id,
                            game.players.get(player_id).unwrap().hand
                        );
                        return Ok((
                            PlayGameReply {
                                request_id: req.request_id.clone(),
                                reply: Some(service::play_game_reply::Reply::MoveReply(
                                    MoveReply {
                                        hand: to_hand(&game.players.get(player_id).unwrap().hand),
                                        board: Some(to_board(&game.board)),
                                        game_over: gr.is_some(),
                                    },
                                )),
                            },
                            Some(PlayGameReply {
                                request_id: "".to_string(),
                                reply: Some(service::play_game_reply::Reply::PlayerMove(
                                    move_to_player_move(
                                        to_board(&game.board),
                                        &game_move,
                                        gr.is_some(),
                                    ),
                                )),
                            }),
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
            Some(play_game_request::Request::PlayRequest(_)) => {
                return Err(error_reply(
                    &req.request_id,
                    "player sent PlayRequest after game had begun",
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

        // let valid_words = Arc::clone(&self.valid_words);
        let mut in_stream = req.into_inner();
        let (tx, rx) = mpsc::channel(128);

        // Okay, the game is on! We expect the first message to be a play request.
        let play_req = match in_stream.next().await {
            Some(Ok(pgr)) => pgr,
            Some(Err(e)) => {
                eprintln!("received error status from client {}", e);
                return Err(Status::failed_precondition("client errored"));
            }
            None => {
                eprintln!("stream is over already?");
                return Err(Status::failed_precondition("no move was given?"));
            }
        };
        let pr = match play_req.request {
            Some(play_game_request::Request::PlayRequest(pr)) => pr,
            Some(_) => {
                return Err(Status::failed_precondition(
                    "initial request was not a play request",
                ))
            }
            None => {
                return Err(Status::failed_precondition(
                    "no actual request in initial request",
                ))
            }
        };

        let gp = GamePlayer {
            id: player_id,
            name: pr.player_name.clone(),
            sender: tx,
            stream: in_stream,
            initial_req_id: play_req.request_id,
        };

        let game_handler = match (player_id, gp_tx, gp_rx) {
            (0, None, Some(rx)) => {
                // If we're player zero, wait to get info from player one.
                let other_player_gp = rx.await.map_err(|e| {
                    Status::internal(format!("failed to get other player info: {:?}", e))
                })?;

                let mut game = Game::new(9, 9, None, GameRules::generation(2));
                game.add_player(gp.name.clone());
                game.add_player(other_player_gp.name.clone());
                game.rules.battle_delay = 0;
                game.board.cache_special_squares();
                game.start();
                Some(GameHandler {
                    players: vec![gp, other_player_gp],
                    game: Arc::new(RwLock::new(game)),
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
                // if let Some(Ok(v)) = in_stream.next().await {
                //     println!("{:?}", v);
                // }
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

// fn test() {
//     let mut player_index = 0;
//     loop {
//         let res = g
//             .play_turn(
//                 Move::Place {
//                     player: player_index,
//                     tile: 'a',
//                     position: Coordinate::new(1, 2),
//                 },
//                 Some(&valid_words),
//                 Some(&valid_words),
//                 None,
//             )
//             .unwrap();
//         println!("{:?}", res);
//         // 0 -> 1 or 1 -> 0
//         //
//         player_index = 1 - player_index;
//     }
// }

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
