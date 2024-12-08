use service::{move_request, player_move};
use std::collections::HashMap;
use truncate_core::{
    board::Coordinate,
    game::Game,
    judge::{WordData, WordDict},
    moves::Move,
};

pub mod service {
    tonic::include_proto!("service");
}

pub static TRUNCATE_DICT: &str = include_str!("../../dict_builder/final_wordlist.txt");

pub fn init_dict() -> anyhow::Result<WordDict> {
    let mut valid_words = HashMap::new();
    let lines = TRUNCATE_DICT.lines();

    for line in lines {
        let mut chunks = line.split(' ');

        let mut word = chunks.next().unwrap().to_string();
        let objectionable = word.chars().next() == Some('*');
        if objectionable {
            word.remove(0);
        }

        valid_words.insert(
            word,
            WordData {
                extensions: chunks.next().unwrap().parse()?,
                rel_freq: chunks.next().unwrap().parse()?,
                objectionable,
            },
        );
    }

    Ok(valid_words)
}

pub fn player_move_to_move(pm: &service::PlayerMove) -> Option<Move> {
    match &pm.r#move {
        Some(player_move::Move::PlaceMove(pmm)) => {
            Some(place_move_to_move(pm.player_id as usize, pmm))
        }
        Some(player_move::Move::SwapMove(sm)) => Some(swap_move_to_move(pm.player_id as usize, sm)),
        None => None,
    }
}

pub fn move_request_to_move(player_id: usize, mr: &service::MoveRequest) -> Option<Move> {
    match &mr.r#move {
        Some(move_request::Move::PlaceMove(pm)) => Some(place_move_to_move(player_id, pm)),
        Some(move_request::Move::SwapMove(sm)) => Some(swap_move_to_move(player_id, sm)),
        None => None,
    }
}

pub fn move_to_player_move(
    board: service::Board,
    mv: &Move,
    game_over: bool,
) -> service::PlayerMove {
    let (player_id, mv) = match mv {
        Move::Place {
            player,
            tile,
            position,
        } => (
            player,
            player_move::Move::PlaceMove(service::PlaceMove {
                tile: tile.to_string(),
                position: to_wire_coord(&position),
            }),
        ),
        Move::Swap { player, positions } => (
            player,
            player_move::Move::SwapMove(service::SwapMove {
                from: to_wire_coord(&positions[0]),
                to: to_wire_coord(&positions[1]),
            }),
        ),
    };
    service::PlayerMove {
        player_id: *player_id as u32,
        board: Some(board),
        r#move: Some(mv),
        game_over,
    }
}

fn to_wire_coord(c: &Coordinate) -> Option<service::Coordinate> {
    Some(service::Coordinate {
        x: c.x as u32,
        y: c.y as u32,
    })
}

pub fn place_move_to_move(player_id: usize, pm: &service::PlaceMove) -> Move {
    let pos = pm.position.unwrap();
    Move::Place {
        player: player_id,
        tile: pm.tile.chars().next().expect("String is empty"),
        position: Coordinate {
            x: pos.x as usize,
            y: pos.y as usize,
        },
    }
}

pub fn swap_move_to_move(player_id: usize, sm: &service::SwapMove) -> Move {
    let from_pos = sm.from.unwrap();
    let to_pos = sm.to.unwrap();
    Move::Swap {
        player: player_id,
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
    }
}

pub fn invert_move(game: &Game, game_move: &Move) -> Move {
    // Possible TODO: This does __not__ handle fog of war coordinate mutations.
    match game_move {
        Move::Place {
            player,
            tile,
            position,
        } => Move::Place {
            player: *player,
            tile: *tile,
            position: game.board.reciprocal_coordinate(*position),
        },
        Move::Swap { player, positions } => Move::Swap {
            player: *player,
            positions: [
                game.board.reciprocal_coordinate(positions[0]),
                game.board.reciprocal_coordinate(positions[1]),
            ],
        },
    }
}
