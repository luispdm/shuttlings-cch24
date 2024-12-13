use core::{
    clone::Clone,
    convert::{From, Into},
    fmt,
};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct BoardState {
    pub board: Arc<Mutex<Board>>,
}

#[derive(Debug)]
pub struct Board {
    tiles: Vec<Vec<Tile>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Tile {
    Empty,
    Team(Team),
    Wall,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Team {
    Cookie,
    Milk,
}

impl fmt::Display for Tile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Tile::Team(t) => match t {
                Team::Cookie => "🍪",
                Team::Milk => "🥛",
            },
            Tile::Empty => "⬛",
            Tile::Wall => "⬜",
        };
        write!(f, "{}", s)
    }
}

impl From<Team> for Tile {
    fn from(value: Team) -> Self {
        match value {
            Team::Cookie => Tile::Team(Team::Cookie),
            Team::Milk => Tile::Team(Team::Milk),
        }
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

    fn game_over(&self) -> bool {
        !self
            .tiles
            .iter()
            .any(|r| r.iter().any(|c| c == &Tile::Empty))
    }

    fn deepest_row(&self, col: &usize) -> Option<usize> {
        self.tiles
            .iter()
            .enumerate()
            .filter(|(_, r)| r[*col] == Tile::Empty)
            .map(|(i, _)| i)
            .collect::<Vec<usize>>()
            .iter()
            .max()
            .copied()
    }

    fn place_team(&mut self, team: &Team, row: &usize, col: &usize) {
        self.tiles[*row][*col] = team.clone().into();
    }

    // TODO need to handle winning team!
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

pub async fn place(
    State(state): State<BoardState>,
    Path((team, column)): Path<(Team, usize)>,
) -> impl IntoResponse {
    // return if team does not exist
    if team != Team::Milk && team != Team::Cookie {
        return (StatusCode::BAD_REQUEST, "".to_string());
    }

    // return if column is out of range
    if !(1..5).contains(&column) {
        return (StatusCode::BAD_REQUEST, "".to_string());
    }

    let mut board = state.board.lock().await;

    // return if game is over
    if board.game_over() {
        return (StatusCode::SERVICE_UNAVAILABLE, board.to_string());
    }

    // try to place the item
    match board.deepest_row(&column) {
        Some(row) => {
            board.place_team(&team, &row, &column);
            (StatusCode::OK, board.to_string())
        }
        _ => (StatusCode::SERVICE_UNAVAILABLE, board.to_string()), // column unavailable
    }
}

pub fn board_state() -> Arc<Mutex<Board>> {
    Arc::new(Mutex::new(Board::new()))
}
