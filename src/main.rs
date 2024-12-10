mod day5;
mod day_2;
mod day_minus_1;

use axum::{
    routing::{get, post},
    Router,
};

use crate::{day5::*, day_2::*, day_minus_1::*};

#[shuttle_runtime::main]
async fn main() -> shuttle_axum::ShuttleAxum {
    let router = Router::new()
        .route("/", get(hello_world))
        .route("/-1/seek", get(seek))
        .route("/2/dest", get(dest_v4))
        .route("/2/key", get(key_v4))
        .route("/2/v6/dest", get(dest_v6))
        .route("/2/v6/key", get(key_v6))
        .route("/5/manifest", post(manifest));

    Ok(router.into())
}
