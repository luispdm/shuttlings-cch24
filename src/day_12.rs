use std::sync::Arc;
use core::fmt;

use axum::{extract::State, http::StatusCode, response::IntoResponse};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct BoardState {
    pub board: Arc<Mutex<Board>>,
}

#[derive(Debug)]
pub struct Board {
    tiles: Vec<Vec<Tile>>,
}

#[derive(Clone, Debug)]
enum Tile {
    Cookie,
    Empty,
    Milk,
    Wall,
}

impl fmt::Display for Tile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Tile::Cookie => "🍪",
            Tile::Milk => "🥛",
            Tile::Empty => "⬛",
            Tile::Wall => "⬜",
        };
        write!(f, "{}", s)
    }
}

impl Board {
    fn new() -> Self {
        let mut b = Board {
            tiles: vec![vec![Tile::Wall; 6]; 5],
        };

        b.tiles = (0..5)
            .map(|i| {
                (0..6)
                    .map(|j| match (i, j) {
                        (0..=3, 0 | 5) => Tile::Wall,
                        (4, _) => Tile::Wall,
                        _ => Tile::Empty,
                    })
                    .collect()
            })
            .collect();
        b
    }
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let board = &self
            .tiles
            .iter()
            .map(|row| {
                row.iter()
                    .map(|tile| tile.to_string())
                    .collect::<Vec<String>>()
                    .join("")
            })
            .collect::<Vec<String>>()
            .join("\n");

        writeln!(f, "{}", board.trim())
    }
}

pub async fn reset_board(State(state): State<BoardState>) -> impl IntoResponse {
    let mut board = state.board.lock().await;
    *board = Board::new();
    (StatusCode::OK, board.to_string())
}

pub async fn board(State(state): State<BoardState>) -> impl IntoResponse {
    (StatusCode::OK, state.board.lock().await.to_string())
}

pub fn board_state() -> Arc<Mutex<Board>> {
    Arc::new(Mutex::new(Board::new()))
}
