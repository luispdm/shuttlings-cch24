mod day_2;
mod day_5;
mod day_9;
mod day_minus_1;

use axum::{
    routing::{get, post},
    Router,
};

use crate::{day_2::*, day_5::*, day_9::*, day_minus_1::*};

#[shuttle_runtime::main]
async fn main() -> shuttle_axum::ShuttleAxum {
    let state = AppState {
        limiter: state_rate_limiter(),
    };

    let router = Router::new()
        .route("/", get(hello_world))
        .route("/-1/seek", get(seek))
        .route("/2/dest", get(dest_v4))
        .route("/2/key", get(key_v4))
        .route("/2/v6/dest", get(dest_v6))
        .route("/2/v6/key", get(key_v6))
        .route("/5/manifest", post(manifest))
        .route("/9/milk", post(milk))
        .route("/9/refill", post(refill))
        .with_state(state);

    Ok(router.into())
}
