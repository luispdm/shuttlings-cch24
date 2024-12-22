use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{query, query_as, FromRow, PgPool};
use uuid::Uuid;

#[derive(Clone)]
pub struct DbState {
    pub pool: PgPool,
}

#[derive(Serialize, FromRow)]
struct Quote {
    id: Uuid,
    author: String,
    quote: String,
    created_at: DateTime<Utc>,
    version: i32,
}

#[derive(Deserialize)]
pub struct NewQuote {
    author: String,
    quote: String,
}

pub async fn cite(Path(id): Path<Uuid>, State(state): State<DbState>) -> impl IntoResponse {
    match query_as::<_, Quote>("SELECT * FROM quotes WHERE id = $1")
        .bind(id)
        .fetch_one(&state.pool)
        .await
    {
        Ok(q) => Ok((StatusCode::OK, q.quote)),
        _ => Err((StatusCode::NOT_FOUND, "".to_string())),
    }
}

pub async fn draft(
    State(state): State<DbState>,
    Json(new_quote): Json<NewQuote>,
) -> impl IntoResponse {
    match query_as::<_, Quote>(
        "INSERT INTO quotes (id, author, quote) VALUES ($1, $2, $3) RETURNING *",
    )
    .bind(Uuid::new_v4())
    .bind(&new_quote.author)
    .bind(&new_quote.quote)
    .fetch_one(&state.pool)
    .await
    {
        Ok(q) => Ok((StatusCode::CREATED, Json(q))),
        _ => Err((StatusCode::NOT_FOUND, "".to_string())),
    }
}

pub async fn reset_quotes(State(state): State<DbState>) -> impl IntoResponse {
    match query("TRUNCATE TABLE quotes").execute(&state.pool).await {
        Ok(_) => Ok(StatusCode::OK),
        _ => Err((StatusCode::INTERNAL_SERVER_ERROR, "".to_string())),
    }
}
