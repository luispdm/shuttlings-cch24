use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{query_as, FromRow, PgPool};
use uuid::Uuid;

#[derive(Clone)]
pub struct DbState {
    pub pool: PgPool,
}

#[derive(Serialize, FromRow)]
struct Quote {
    pub id: Uuid,
    pub author: String,
    pub quote: String,
    pub created_at: DateTime<Utc>,
    pub version: i32,
}

pub async fn cite(Path(id): Path<Uuid>, State(state): State<DbState>) -> impl IntoResponse {
    match query_as::<_, Quote>("SELECT * FROM quotes WHERE id = $1")
        .bind(id)
        .fetch_one(&state.pool)
        .await
    {
        Ok(quote) => Ok((StatusCode::OK, Json(quote))), // TODO probably need to return only `quote`
        _ => Err((StatusCode::NOT_FOUND, "".to_string())),
    }
}

pub async fn reset_quotes() -> impl IntoResponse {
    // TODO to implement
    StatusCode::OK
}
