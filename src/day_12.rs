use core::fmt;

use axum::{http::StatusCode, response::IntoResponse};

#[derive(Debug)]
struct Board {
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

        writeln!(f, "{}", board)
    }
}

pub async fn reset_board() -> impl IntoResponse {
    (StatusCode::OK, Board::new().to_string())
}

pub async fn board() -> impl IntoResponse {
    // TODO change me and wrap board in a Mutex - need AppState
    (StatusCode::OK, Board::new().to_string())
}
