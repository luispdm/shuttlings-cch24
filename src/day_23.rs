use axum::{http::StatusCode, response::IntoResponse};

pub async fn star() -> impl IntoResponse {
    (StatusCode::OK, "<div id=\"star\" class=\"lit\"></div>")
}
