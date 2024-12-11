use std::sync::Arc;

use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use leaky_bucket::RateLimiter;
use serde::{Deserialize, Serialize};

const INITIAL_TOKENS: usize = 5;
const MAX_TOKENS: usize = 5;
const REFILL_INTERVAL: u64 = 1;
const REFILL_AMOUNT: usize = 1;

pub struct AppState {
    pub limiter: RateLimiter,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Milk {
    Liters(f32),
    Gallons(f32),
    Litres(f32),
    Pints(f32),
}

impl Milk {
    fn convert(&self) -> Self {
        match self {
            Milk::Liters(l) => Self::Gallons(*l * 0.264172),
            Milk::Gallons(g) => Self::Liters(*g * 3.78541),
            Milk::Litres(l) => Self::Pints(*l * 1.75975),
            Milk::Pints(p) => Self::Litres(*p * 0.56826),
        }
    }
}

pub async fn milk(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    if !state.limiter.try_acquire(1) {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            "No milk available\n".to_string(),
        );
    }

    // parse content-type header
    let ct = match headers.get("Content-Type") {
        Some(ct) if ct == "application/json" => ct.to_str().ok(),
        _ => None,
    };

    // return the default string if no content-type is provided or try the conversion
    if ct.is_none() {
        (StatusCode::OK, "Milk withdrawn\n".to_string())
    } else {
        serde_json::from_slice::<Milk>(&body)
            .map(|m| {
                (
                    StatusCode::OK,
                    serde_json::ser::to_string(&m.convert()).unwrap(),
                )
            })
            .unwrap_or_else(|_| (StatusCode::BAD_REQUEST, "".to_string()))
    }
}

pub fn rate_limiter() -> RateLimiter {
    RateLimiter::builder()
        .initial(INITIAL_TOKENS)
        .interval(tokio::time::Duration::from_secs(REFILL_INTERVAL))
        .refill(REFILL_AMOUNT)
        .max(MAX_TOKENS)
        .build()
}
