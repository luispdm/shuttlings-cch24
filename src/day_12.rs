use core::{
    clone::Clone,
    convert::{From, Into},
    fmt,
    iter::Iterator,
    ops::RangeInclusive,
    option::Option,
    unreachable, writeln,
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
    winner: Option<Winner>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Tile {
    Empty,
    Team(Team),
    Wall,
}

#[derive(Debug)]
enum Winner {
    Team(Team),
    Tie,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
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

impl fmt::Display for Winner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Winner::Team(t) => format!("{} wins!", Tile::Team(*t)),
            Winner::Tie => "No winner.".to_string(),
        };
        write!(f, "{}", s)
    }
}

impl From<Team> for Tile {
    fn from(value: Team) -> Self {
        match value {
            Team::Cookie => Self::Team(Team::Cookie),
            Team::Milk => Self::Team(Team::Milk),
        }
    }
}

impl From<Tile> for Winner {
    fn from(value: Tile) -> Self {
        match value {
            Tile::Team(t) => Self::Team(t),
            // winner should be printed only if it's cookie or milk
            _ => unreachable!(),
        }
    }
}

impl Board {
    fn new() -> Self {
        let mut b = Board {
            tiles: vec![vec![Tile::Wall; Board::columns()]; Board::rows()],
            winner: None,
        };

        b.tiles = (0..Board::rows())
            .map(|i| {
                (0..Board::columns())
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

    fn board_full(&self) -> bool {
        !self
            .tiles
            .iter()
            .any(|r| r.iter().any(|c| c == &Tile::Empty))
    }

    fn deepest_row(&self, col: &usize) -> Option<usize> {
        self.tiles
            .iter()
            .enumerate()
            .rev()
            .find(|(_, r)| r[*col] == Tile::Empty)
            .map(|(i, _)| i)
    }

    fn place_team(&mut self, team: &Team, row: &usize, col: &usize) {
        self.tiles[*row][*col] = Tile::from(*team);
    }

    fn set_winner(&mut self) {
        // check if there are 4 equal elements on any row
        self.winner = self.winner_on_row();
        if self.winner.is_some() {
            return;
        }

        // check if there are 4 equal elements on any column
        self.winner = self.winner_on_column();
        if self.winner.is_some() {
            return;
        }

        // check if there are 4 equal elements on the first diagonal
        self.winner = self.winner_on_diagonal();
        if self.winner.is_some() {
            return;
        }

        // no winner - if the board is full, it's a tie
        if self.board_full() {
            self.winner = Some(Winner::Tie);
        }
    }

    fn winner_on_row(&self) -> Option<Winner> {
        let playable_tiles = Board::playable_rows()
            .map(|row| &self.tiles[row][Board::playable_columns()])
            .collect::<Vec<_>>();

        playable_tiles
            .iter()
            .find(|row| {
                row.iter().all(|tile| match tile {
                    Tile::Team(_) => row.iter().all(|t| t == tile),
                    _ => false,
                })
            })
            .and_then(|winning_row| match &winning_row[0] {
                Tile::Team(team) => Some(Winner::Team(*team)),
                _ => None,
            })
    }

    fn winner_on_column(&self) -> Option<Winner> {
        Board::playable_columns()
            .find(|col| {
                let column_tiles: Vec<&Tile> = Board::playable_rows()
                    .map(|row| &self.tiles[row][*col])
                    .collect();

                column_tiles.iter().all(|&tile| tile == column_tiles[0])
                    && column_tiles[0] != &Tile::Empty
            })
            .and_then(|winning_col| match &self.tiles[0][winning_col] {
                Tile::Team(team) => Some(Winner::Team(*team)),
                _ => None,
            })
    }

    fn winner_on_diagonal(&self) -> Option<Winner> {
        let first_diagonal: Vec<&Tile> = Board::playable_rows()
            .zip(Board::playable_columns())
            .map(|(row, col)| &self.tiles[row][col])
            .collect();
        if first_diagonal.iter().all(|&tile| tile == first_diagonal[0])
            && *first_diagonal[0] != Tile::Empty
        {
            return Some(Winner::from(*first_diagonal[0]));
        }

        let last_diagonal: Vec<&Tile> = Board::playable_rows()
            .zip(Board::playable_columns().rev())
            .map(|(row, col)| &self.tiles[row][col])
            .collect();
        if last_diagonal.iter().all(|&tile| tile == last_diagonal[0])
            && *last_diagonal[0] != Tile::Empty
        {
            return Some(Winner::from(*last_diagonal[0]));
        }

        None
    }

    fn rows() -> usize {
        5
    }

    fn columns() -> usize {
        6
    }

    fn playable_rows() -> RangeInclusive<usize> {
        RangeInclusive::new(0, 3)
    }

    fn playable_columns() -> RangeInclusive<usize> {
        RangeInclusive::new(1, 4)
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

        match &self.winner {
            Some(w) => writeln!(f, "{}\n{}", board.trim(), w),
            _ => writeln!(f, "{}", board.trim()),
        }
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
    if !Board::playable_columns().contains(&column) {
        return (StatusCode::BAD_REQUEST, "".to_string());
    }

    let mut board = state.board.lock().await;

    // return if game is over
    if board.winner.is_some() {
        return (StatusCode::SERVICE_UNAVAILABLE, board.to_string());
    }

    // try to place the item
    match board.deepest_row(&column) {
        Some(row) => {
            board.place_team(&team, &row, &column);
            board.set_winner();
            (StatusCode::OK, board.to_string())
        }
        // column unavailable
        _ => (StatusCode::SERVICE_UNAVAILABLE, board.to_string()),
    }
}

pub fn board_state() -> Arc<Mutex<Board>> {
    Arc::new(Mutex::new(Board::new()))
}
