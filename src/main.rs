mod day_12;
mod day_16;
mod day_19;
mod day_2;
mod day_5;
mod day_9;
mod day_minus_1;

use axum::{
    routing::{delete, get, post, put},
    Router,
};
use sqlx::PgPool;

use crate::{day_12::*, day_16::*, day_19::*, day_2::*, day_5::*, day_9::*, day_minus_1::*};

#[shuttle_runtime::main]
async fn main(#[shuttle_shared_db::Postgres] pool: PgPool) -> shuttle_axum::ShuttleAxum {
    sqlx::migrate!("src/day_19")
        .run(&pool)
        .await
        .expect("Failed to run day 19 migrations");

    let db_state = DbState {
        pool,
        tokens: state_tokens(),
    };

    let rate_limiter_state = RateLimiterState {
        limiter: state_rate_limiter(),
    };

    let board_state = BoardState {
        board: arc_board(),
        random_board: arc_random_board(),
    };

    let router = Router::new()
        .route("/", get(hello_bird))
        .route("/-1/seek", get(seek))
        .route("/2/dest", get(dest_v4))
        .route("/2/key", get(key_v4))
        .route("/2/v6/dest", get(dest_v6))
        .route("/2/v6/key", get(key_v6))
        .route("/5/manifest", post(manifest))
        .route("/9/milk", post(milk))
        .route("/9/refill", post(refill))
        .with_state(rate_limiter_state)
        .route("/12/board", get(board))
        .route("/12/random-board", get(random))
        .route("/12/reset", post(reset))
        .route("/12/place/:team/:column", post(place))
        .with_state(board_state)
        .route("/16/wrap", post(wrap))
        .route("/16/unwrap", get(unwrap))
        .route("/16/decode", post(decode))
        .route("/19/reset", post(reset_quotes))
        .route("/19/cite/:id", get(cite))
        .route("/19/draft", post(draft))
        .route("/19/remove/:id", delete(remove))
        .route("/19/undo/:id", put(undo))
        .route("/19/list", get(list))
        .with_state(db_state);

    Ok(router.into())
}
