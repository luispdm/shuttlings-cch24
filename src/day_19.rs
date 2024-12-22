use std::{collections::HashMap, sync::Arc};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use axum_extra::extract::OptionalQuery;
use chrono::{DateTime, Utc};
use rand::distributions::DistString;
use serde::{Deserialize, Serialize};
use sqlx::{query, query_as, query_scalar, FromRow, PgPool};
use tokio::sync::Mutex;
use uuid::Uuid;

const PAGE_SIZE: i64 = 3;

#[derive(Clone)]
pub struct DbState {
    pub pool: PgPool,
    pub tokens: Arc<Mutex<HashMap<String, i64>>>,
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

#[derive(Deserialize)]
pub struct Token {
    token: String,
}

#[derive(Serialize, FromRow)]
struct Quotes {
    quotes: Vec<Quote>,
    page: i64,
    next_token: Option<String>,
}

pub async fn cite(Path(id): Path<Uuid>, State(state): State<DbState>) -> impl IntoResponse {
    match query_as::<_, Quote>("SELECT * FROM quotes WHERE id = $1")
        .bind(id)
        .fetch_one(&state.pool)
        .await
    {
        Ok(q) => Ok((StatusCode::OK, Json(q))),
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

pub async fn remove(Path(id): Path<Uuid>, State(state): State<DbState>) -> impl IntoResponse {
    match query_as::<_, Quote>("DELETE FROM quotes WHERE id = $1 RETURNING *")
        .bind(id)
        .fetch_one(&state.pool)
        .await
    {
        Ok(q) => Ok((StatusCode::OK, Json(q))),
        _ => Err((StatusCode::NOT_FOUND, "".to_string())),
    }
}

pub async fn undo(
    Path(id): Path<Uuid>,
    State(state): State<DbState>,
    Json(new_quote): Json<NewQuote>,
) -> impl IntoResponse {
    match query_as::<_, Quote>(
        "UPDATE quotes SET author = $2, quote = $3, version = version + 1 WHERE id = $1 RETURNING *",
    )
    .bind(id)
    .bind(&new_quote.author)
    .bind(&new_quote.quote)
    .fetch_one(&state.pool)
    .await
    {
        Ok(q) => Ok((StatusCode::OK, Json(q))),
        _ => Err((StatusCode::NOT_FOUND, "".to_string())),
    }
}

pub async fn reset_quotes(State(state): State<DbState>) -> impl IntoResponse {
    match query("TRUNCATE TABLE quotes").execute(&state.pool).await {
        Ok(_) => Ok(StatusCode::OK),
        _ => Err((StatusCode::INTERNAL_SERVER_ERROR, "".to_string())),
    }
}

pub async fn list(token: OptionalQuery<Token>, State(state): State<DbState>) -> impl IntoResponse {
    let mut tokens = state.tokens.lock().await;

    let page = if token.is_none() {
        // if no token is given, fetch the first page
        1
    } else if let Some(p) = tokens.get(token.0.unwrap().token.as_str()) {
        // if the token is valid, fetch the desired page
        *p
    } else {
        // token not found, user error
        return Err((StatusCode::BAD_REQUEST, "".to_string()));
    };

    let total_pages = match total_pages(&state.pool).await {
        Ok(p) => p,
        Err(e) => return Err(e),
    };

    let quotes = match page_quotes(&state.pool, page).await {
        Ok(q) => q,
        Err(e) => return Err(e),
    };

    let next_token = if page < total_pages {
        let n = rand::distributions::Alphanumeric.sample_string(&mut rand::thread_rng(), 16);
        tokens.insert(n.clone(), page + 1);
        Some(n)
    } else {
        None
    };

    Ok((
        StatusCode::OK,
        Json(Quotes {
            quotes,
            page,
            next_token,
        }),
    ))
}

async fn total_pages(pool: &PgPool) -> Result<i64, (StatusCode, String)> {
    match query_scalar::<_, i64>("SELECT COUNT(*) FROM quotes")
        .fetch_one(pool)
        .await
    {
        Ok(count) => Ok((count as f64 / PAGE_SIZE as f64).ceil() as i64),
        _ => Err((StatusCode::INTERNAL_SERVER_ERROR, "".to_string())),
    }
}

async fn page_quotes(pool: &PgPool, page: i64) -> Result<Vec<Quote>, (StatusCode, String)> {
    match query_as::<_, Quote>("SELECT * FROM quotes ORDER BY created_at OFFSET $1 LIMIT $2")
        .bind((page - 1) * PAGE_SIZE)
        .bind(PAGE_SIZE)
        .fetch_all(pool)
        .await
    {
        Ok(quotes) => Ok(quotes),
        _ => Err((StatusCode::INTERNAL_SERVER_ERROR, "".to_string())),
    }
}

pub fn state_tokens() -> Arc<Mutex<HashMap<String, i64>>> {
    Arc::new(Mutex::new(HashMap::new()))
}
