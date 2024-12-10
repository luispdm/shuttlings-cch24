use std::sync::Arc;

use axum::{extract::State, http::StatusCode, response::IntoResponse};
use leaky_bucket::RateLimiter;

const INITIAL_TOKENS: usize = 5;
const MAX_TOKENS: usize = 5;
const REFILL_INTERVAL: u64 = 1;
const REFILL_AMOUNT: usize = 1;

pub struct AppState {
    pub limiter: RateLimiter,
}

pub async fn milk(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    StatusCode::OK
}

pub fn rate_limiter() -> RateLimiter {
    RateLimiter::builder()
        .initial(INITIAL_TOKENS)
        .interval(tokio::time::Duration::from_secs(REFILL_INTERVAL))
        .refill(REFILL_AMOUNT)
        .max(MAX_TOKENS)
        .build()
}
