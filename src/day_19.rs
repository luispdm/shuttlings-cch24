use std::{collections::HashMap, sync::Arc};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use axum_extra::extract::OptionalQuery;
use chrono::{DateTime, Utc};
#[cfg(test)]
use mockall::{automock, predicate::*};
use rand::distributions::DistString;
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgQueryResult, query, query_as, query_scalar, FromRow, PgPool};
use tokio::sync::Mutex;
use uuid::Uuid;

const PAGE_SIZE: i64 = 3;

#[derive(Clone)]
pub struct DbState {
    pub repository: Arc<dyn QuoteRepository>,
    pub tokens: Arc<Mutex<HashMap<String, i64>>>,
}

#[derive(Clone, Deserialize, Serialize, FromRow)]
pub struct Quote {
    id: Uuid,
    author: String,
    quote: String,
    created_at: DateTime<Utc>,
    version: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NewQuote {
    author: String,
    quote: String,
}

#[derive(Deserialize)]
pub struct Token {
    token: String,
}

#[derive(Deserialize, Serialize, FromRow)]
struct Quotes {
    quotes: Vec<Quote>,
    page: i64,
    next_token: Option<String>,
}

#[async_trait::async_trait]
#[cfg_attr(test, automock)]
pub trait QuoteRepository: Send + Sync + 'static {
    async fn get(&self, id: Uuid) -> Result<Quote, sqlx::Error>;
    async fn create(&self, new_quote: NewQuote) -> Result<Quote, sqlx::Error>;
    async fn delete(&self, id: Uuid) -> Result<Quote, sqlx::Error>;
    async fn update(&self, id: Uuid, new_quote: NewQuote) -> Result<Quote, sqlx::Error>;
    async fn get_quotes(&self, offset: i64, limit: i64) -> Result<Vec<Quote>, sqlx::Error>;
    async fn count_quotes(&self) -> Result<i64, sqlx::Error>;
    async fn reset_quotes(&self) -> Result<PgQueryResult, sqlx::Error>;
}

pub struct PostgresQuoteRepository {
    pool: PgPool,
}

impl PostgresQuoteRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl QuoteRepository for PostgresQuoteRepository {
    async fn get(&self, id: Uuid) -> Result<Quote, sqlx::Error> {
        query_as::<_, Quote>("SELECT * FROM quotes WHERE id = $1")
            .bind(id)
            .fetch_one(&self.pool)
            .await
    }

    async fn create(&self, new_quote: NewQuote) -> Result<Quote, sqlx::Error> {
        query_as::<_, Quote>(
            "INSERT INTO quotes (id, author, quote) VALUES ($1, $2, $3) RETURNING *",
        )
        .bind(Uuid::new_v4())
        .bind(&new_quote.author)
        .bind(&new_quote.quote)
        .fetch_one(&self.pool)
        .await
    }

    async fn delete(&self, id: Uuid) -> Result<Quote, sqlx::Error> {
        query_as::<_, Quote>("DELETE FROM quotes WHERE id = $1 RETURNING *")
            .bind(id)
            .fetch_one(&self.pool)
            .await
    }

    async fn update(&self, id: Uuid, new_quote: NewQuote) -> Result<Quote, sqlx::Error> {
        query_as::<_, Quote>(
            "UPDATE quotes SET author = $2, quote = $3, version = version + 1 WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(&new_quote.author)
        .bind(&new_quote.quote)
        .fetch_one(&self.pool)
        .await
    }

    async fn get_quotes(&self, offset: i64, limit: i64) -> Result<Vec<Quote>, sqlx::Error> {
        query_as::<_, Quote>("SELECT * FROM quotes ORDER BY created_at OFFSET $1 LIMIT $2")
            .bind(offset)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
    }

    async fn count_quotes(&self) -> Result<i64, sqlx::Error> {
        query_scalar::<_, i64>("SELECT COUNT(*) FROM quotes")
            .fetch_one(&self.pool)
            .await
    }

    async fn reset_quotes(&self) -> Result<PgQueryResult, sqlx::Error> {
        query("TRUNCATE TABLE quotes").execute(&self.pool).await
    }
}

pub async fn cite(Path(id): Path<Uuid>, State(state): State<DbState>) -> impl IntoResponse {
    match state.repository.get(id).await {
        Ok(q) => Ok((StatusCode::OK, Json(q))),
        _ => Err((StatusCode::NOT_FOUND, "".to_string())),
    }
}

pub async fn draft(
    State(state): State<DbState>,
    Json(new_quote): Json<NewQuote>,
) -> impl IntoResponse {
    match state.repository.create(new_quote).await {
        Ok(q) => Ok((StatusCode::CREATED, Json(q))),
        _ => Err((StatusCode::NOT_FOUND, "".to_string())),
    }
}

pub async fn remove(Path(id): Path<Uuid>, State(state): State<DbState>) -> impl IntoResponse {
    match state.repository.delete(id).await {
        Ok(q) => Ok((StatusCode::OK, Json(q))),
        _ => Err((StatusCode::NOT_FOUND, "".to_string())),
    }
}

pub async fn undo(
    Path(id): Path<Uuid>,
    State(state): State<DbState>,
    Json(new_quote): Json<NewQuote>,
) -> impl IntoResponse {
    match state.repository.update(id, new_quote).await {
        Ok(q) => Ok((StatusCode::OK, Json(q))),
        _ => Err((StatusCode::NOT_FOUND, "".to_string())),
    }
}

pub async fn reset_quotes(State(state): State<DbState>) -> impl IntoResponse {
    match state.repository.reset_quotes().await {
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

    let total_pages = match total_pages(&state).await {
        Ok(p) => p,
        Err(e) => return Err(e),
    };

    let quotes = match page_quotes(&state, page).await {
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

async fn total_pages(state: &DbState) -> Result<i64, (StatusCode, String)> {
    match state.repository.count_quotes().await {
        Ok(count) => Ok((count as f64 / PAGE_SIZE as f64).ceil() as i64),
        _ => Err((StatusCode::INTERNAL_SERVER_ERROR, "".to_string())),
    }
}

async fn page_quotes(state: &DbState, page: i64) -> Result<Vec<Quote>, (StatusCode, String)> {
    match state
        .repository
        .get_quotes((page - 1) * PAGE_SIZE, PAGE_SIZE)
        .await
    {
        Ok(quotes) => Ok(quotes),
        _ => Err((StatusCode::INTERNAL_SERVER_ERROR, "".to_string())),
    }
}

pub fn state_tokens() -> Arc<Mutex<HashMap<String, i64>>> {
    Arc::new(Mutex::new(HashMap::new()))
}

pub fn state_repository(pool: PgPool) -> Arc<dyn QuoteRepository> {
    Arc::new(PostgresQuoteRepository::new(pool))
}

#[cfg(test)]
mod tests {
    use core::{
        future::{ready, Future},
        pin::Pin,
    };

    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        response::Response,
        routing::{delete, get, post, put},
        Router,
    };
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    async fn get_response_parts(response: Response) -> (StatusCode, Option<String>) {
        let status = response.status();

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = if !body.is_empty() {
            Some(String::from_utf8(body.to_vec()).unwrap())
        } else {
            None
        };

        (status, body_str)
    }

    fn create_test_app(repository: Arc<dyn QuoteRepository>) -> Router {
        let state = DbState {
            repository,
            tokens: Arc::new(Mutex::new(HashMap::new())),
        };

        Router::new()
            .route("/cite/:id", get(cite))
            .route("/draft", post(draft))
            .route("/remove/:id", delete(remove))
            .route("/undo/:id", put(undo))
            .route("/list", get(list))
            .route("/reset", post(reset_quotes))
            .with_state(state)
    }

    fn box_future<T>(value: T) -> Pin<Box<dyn Future<Output = T> + Send>>
    where
        T: Send + 'static,
    {
        Box::pin(ready(value))
    }

    #[tokio::test]
    async fn test_cite_ok() {
        let mut mock = MockQuoteRepository::new();
        let quote_id = Uuid::new_v4();
        let expected_quote = Quote {
            id: quote_id,
            author: "Test Author".to_string(),
            quote: "Test Quote".to_string(),
            created_at: Utc::now(),
            version: 1,
        };

        mock.expect_get()
            .with(eq(quote_id))
            .returning(move |_| box_future(Ok(expected_quote.clone())));

        let app = create_test_app(Arc::new(mock));

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/cite/{}", quote_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let (status, body_str) = get_response_parts(response).await;
        assert_eq!(status, StatusCode::OK);

        let response_quote: Quote = serde_json::from_str(&body_str.unwrap()).unwrap();
        assert_eq!(response_quote.id, quote_id);
    }

    #[tokio::test]
    async fn test_draft_ok() {
        let mut mock = MockQuoteRepository::new();
        let new_quote = NewQuote {
            author: "New Author".to_string(),
            quote: "New Quote".to_string(),
        };

        mock.expect_create()
            .with(eq(new_quote.clone()))
            .returning(|q| {
                box_future(Ok(Quote {
                    id: Uuid::new_v4(),
                    author: q.author,
                    quote: q.quote,
                    created_at: Utc::now(),
                    version: 1,
                }))
            });

        let app = create_test_app(Arc::new(mock));

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/draft")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&new_quote).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let (status, _) = get_response_parts(response).await;
        assert_eq!(status, StatusCode::CREATED);
    }

    #[tokio::test]
    async fn test_list_first_page_ok() {
        let mut mock = MockQuoteRepository::new();
        let quotes = vec![Quote {
            id: Uuid::new_v4(),
            author: "Author 1".to_string(),
            quote: "Quote 1".to_string(),
            created_at: Utc::now(),
            version: 1,
        }];

        mock.expect_count_quotes().returning(|| box_future(Ok(1)));

        mock.expect_get_quotes()
            .with(eq(0), eq(PAGE_SIZE))
            .returning(move |_, _| box_future(Ok(quotes.clone())));

        let app = create_test_app(Arc::new(mock));

        let response = app
            .oneshot(Request::builder().uri("/list").body(Body::empty()).unwrap())
            .await
            .unwrap();

        let (status, body_str) = get_response_parts(response).await;
        assert_eq!(status, StatusCode::OK);

        let response_quotes: Quotes = serde_json::from_str(&body_str.unwrap()).unwrap();
        assert_eq!(response_quotes.quotes.len(), 1);
        assert_eq!(response_quotes.page, 1);
        assert_eq!(response_quotes.next_token, None);
    }

    #[tokio::test]
    async fn test_remove_ok() {
        let mut mock = MockQuoteRepository::new();
        let quote_id = Uuid::new_v4();
        let quote = Quote {
            id: quote_id,
            author: "Author".to_string(),
            quote: "Quote".to_string(),
            created_at: Utc::now(),
            version: 1,
        };

        mock.expect_delete()
            .with(eq(quote_id))
            .returning(move |_| box_future(Ok(quote.clone())));

        let app = create_test_app(Arc::new(mock));

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/remove/{}", quote_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let (status, body_str) = get_response_parts(response).await;
        assert_eq!(status, StatusCode::OK);

        let response_quote: Quote = serde_json::from_str(&body_str.unwrap()).unwrap();
        assert_eq!(response_quote.id, quote_id);
    }

    #[tokio::test]
    async fn test_undo_ok() {
        let mut mock = MockQuoteRepository::new();
        let quote_id = Uuid::new_v4();
        let new_quote = NewQuote {
            author: "Updated Author".to_string(),
            quote: "Updated Quote".to_string(),
        };

        mock.expect_update()
            .with(eq(quote_id), eq(new_quote.clone()))
            .returning(|id, q| {
                box_future(Ok(Quote {
                    id,
                    author: q.author,
                    quote: q.quote,
                    created_at: Utc::now(),
                    version: 2,
                }))
            });

        let app = create_test_app(Arc::new(mock));

        let response = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/undo/{}", quote_id))
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&new_quote).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let (status, body_str) = get_response_parts(response).await;
        assert_eq!(status, StatusCode::OK);

        let response_quote: Quote = serde_json::from_str(&body_str.unwrap()).unwrap();
        assert_eq!(response_quote.id, quote_id);
        assert_eq!(response_quote.author, new_quote.author);
        assert_eq!(response_quote.quote, new_quote.quote);
        assert_eq!(response_quote.version, 2);
    }

    #[tokio::test]
    async fn test_reset_ok() {
        let mut mock = MockQuoteRepository::new();

        mock.expect_reset_quotes()
            .returning(|| box_future(Ok(PgQueryResult::default())));

        let app = create_test_app(Arc::new(mock));

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/reset")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let (status, _) = get_response_parts(response).await;
        assert_eq!(status, StatusCode::OK);
    }
}
