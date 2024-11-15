use tokio::time::sleep;
use truncate_auto::service::{self, tile, SquareValidity};

use service::truncate_server::{Truncate, TruncateServer};
use service::{move_request, play_game_request};
use service::{ErrorReply, MoveReply, PlayGameReply, PlayGameRequest};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{error::Error, io::ErrorKind, net::ToSocketAddrs, pin::Pin};
use tokio::sync::{mpsc, Notify};
use tokio_stream::{wrappers::ReceiverStream, Stream, StreamExt};
use tonic::{transport::Server, Response, Status};
use truncate_core::board::{Board, Square};
use truncate_core::player::Hand;
use truncate_core::{
    board::Coordinate, game::Game, judge::WordData, moves::Move, rules::GameRules,
};

pub struct AutoServer {
    pending_game: Arc<Mutex<Option<Arc<Mutex<Game>>>>>,
    pending_name_notify: Arc<Mutex<Option<Arc<Notify>>>>,
    valid_words: Arc<HashMap<String, WordData>>,
    pairing: Notify,
}

type ResponseStream = Pin<Box<dyn Stream<Item = Result<PlayGameReply, Status>> + Send>>;

struct GameLoop {
    valid_words: Arc<HashMap<String, WordData>>,
    game: Arc<Mutex<Game>>,
    player_id: usize,
}

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

impl GameLoop {
    fn handle_move(&self, req: PlayGameRequest) -> Result<PlayGameReply, Status> {
        let mut game = self.game.lock().unwrap();
        match game.next_player {
            Some(next_player) => {
                if next_player == self.player_id {
                    // Do the move,
                    match self.play_game_request_to_move(&req) {
                        Ok(game_move) => {
                            let game_resp = game.play_turn(
                                game_move,
                                Some(&self.valid_words),
                                Some(&self.valid_words),
                                None,
                            );
                            match game_resp {
                                Ok(gr) => {
                                    return Ok(PlayGameReply {
                                        request_id: req.request_id.clone(),
                                        reply: Some(service::play_game_reply::Reply::MoveReply(
                                            MoveReply {
                                                hand: to_hand(
                                                    &game.players.get(self.player_id).unwrap().hand,
                                                ),
                                                board: Some(to_board(&game.board)),
                                                game_over: gr.is_some(),
                                            },
                                        )),
                                    })
                                }
                                Err(msg) => return Ok(error_reply(req.request_id, msg)),
                            }
                        }
                        Err(resp) => {
                            return Ok(resp);
                        }
                    }
                } else {
                    return Ok(error_reply(req.request_id, "it's not your turn to play!"));
                }
            }
            None => {
                return Err(Status::internal("no next_player?"));
            }
        }
    }
    fn play_game_request_to_move(&self, req: &PlayGameRequest) -> Result<Move, PlayGameReply> {
        match &req.request {
            Some(play_game_request::Request::PlayRequest(_)) => {
                return Err(error_reply(
                    &req.request_id,
                    "player sent PlayRequest after game had begun",
                ))
            }
            Some(play_game_request::Request::MoveRequest(mr)) => match &mr.r#move {
                Some(move_request::Move::PlaceMove(pm)) => {
                    let pos = pm.position.unwrap();
                    return Ok(Move::Place {
                        player: self.player_id,
                        tile: pm.tile.chars().next().expect("String is empty"),
                        position: Coordinate {
                            x: pos.x as usize,
                            y: pos.y as usize,
                        },
                    });
                }
                Some(move_request::Move::SwapMove(sm)) => {
                    let from_pos = sm.from.unwrap();
                    let to_pos = sm.to.unwrap();
                    return Ok(Move::Swap {
                        player: self.player_id,
                        positions: [
                            Coordinate {
                                x: from_pos.x as usize,
                                y: from_pos.y as usize,
                            },
                            Coordinate {
                                x: to_pos.x as usize,
                                y: to_pos.y as usize,
                            },
                        ],
                    });
                }
                None => {
                    return Err(error_reply(
                        &req.request_id,
                        "No move was given in MoveRequest",
                    ))
                }
            },
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
        let (game, pending_name_notify, player_id) = {
            let mut pending_game = self
                .pending_game
                .lock()
                .map_err(|e| Status::internal(format!("failed to local pending game ID: {}", e)))?;
            let mut pending_name_notify = self.pending_name_notify.lock().map_err(|e| {
                Status::internal(format!("failed to local pending name notify: {}", e))
            })?;
            match (pending_game.take(), pending_name_notify.take()) {
                (Some(game), Some(pending_name_notify)) => (game, pending_name_notify, 1),
                (None, None) => {
                    let g = Arc::new(Mutex::new(Game::new(9, 9, None, GameRules::generation(1))));
                    let n = Arc::new(Notify::new());
                    *pending_game = Some(Arc::clone(&g));
                    *pending_name_notify = Some(Arc::clone(&n));
                    (g, n, 0)
                }
                (pg, pnn) => {
                    return Err(Status::internal(format!(
                        "expected game and pending_name_notify to be in sync ({}, {})",
                        pg.is_some(),
                        pnn.is_some()
                    )))
                }
            }
        };

        match player_id {
            // Wait for a second player to join.
            0 => self.pairing.notified().await,
            // Notify the first player someone has joined.
            1 => self.pairing.notify_one(),
            _ => {
                return Err(Status::internal(format!(
                    "unexpected player id {}",
                    player_id
                )))
            }
        }

        let valid_words = Arc::clone(&self.valid_words);
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

        // Not sure if this synchronization will actually work the way we want,
        // but we'll figure it out.
        // Specifically, if the player names end up swapped, that bug is here.
        match player_id {
            0 => {
                // Add name before waiting.
                game.lock().unwrap().add_player(pr.player_name);
                // Wait for a second player to join.
                pending_name_notify.notified().await;
            }
            1 => {
                // Notify the first player someone has joined.
                pending_name_notify.notify_one();
                // Add name after notifying
                game.lock().unwrap().add_player(pr.player_name);
            }
            _ => {
                return Err(Status::internal(format!(
                    "unexpected player id {}",
                    player_id
                )))
            }
        }

        // TODO: Actually fix the logic above
        sleep(Duration::from_millis(100)).await;

        let reply = {
            let game = game.lock().unwrap();

            PlayGameReply {
                request_id: play_req.request_id,
                reply: Some(service::play_game_reply::Reply::PlayReply(
                    service::PlayReply {
                        player_id: player_id as u32,
                        hand: to_hand(&game.players.get(player_id).unwrap().hand),
                        board: Some(to_board(&game.board)),
                        opponents: game
                            .players
                            .iter()
                            .enumerate()
                            .filter(|(i, _v)| *i != player_id)
                            .map(|(_i, v)| service::Player {
                                id: player_id as u32,
                                name: v.name.clone(),
                            })
                            .collect(),
                        first_to_move: 0,
                    },
                )),
            }
        };

        if let Err(err) = tx.send(Ok(reply)).await {
            return Err(Status::internal(format!(
                "failed to send reply to client: {}",
                err
            )));
        }

        let game_loop = GameLoop {
            valid_words,
            game,
            player_id,
        };

        // this spawn here is required if you want to handle connection error.
        // If we just map `in_stream` and write it back as `out_stream` the `out_stream`
        // will be dropped when connection error occurs and error will never be propagated
        // to mapped version of `in_stream`.
        tokio::spawn(async move {
            while let Some(result) = in_stream.next().await {
                let msg = match result {
                    Ok(v) => game_loop.handle_move(v),
                    Err(err) => {
                        if let Some(io_err) = match_for_io_error(&err) {
                            if io_err.kind() == ErrorKind::BrokenPipe {
                                // here you can handle special case when client
                                // disconnected in unexpected way
                                eprintln!("\tclient disconnected: broken pipe");
                                break;
                            }
                        }
                        Err(err)
                    }
                };
                if let Err(_err) = tx.send(msg).await {
                    // Response was dropped
                    break;
                }
            }
            println!("\tstream ended");
        });

        let out_stream = ReceiverStream::new(rx);

        Ok(Response::new(Box::pin(out_stream) as Self::PlayGameStream))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let valid_words = truncate_auto::init_dict()?;

    let server = AutoServer {
        pending_game: Arc::new(Mutex::new(None)),
        pending_name_notify: Arc::new(Mutex::new(None)),
        valid_words: Arc::new(valid_words),
        pairing: Notify::new(),
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
