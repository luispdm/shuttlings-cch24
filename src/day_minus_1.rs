use axum::{
    http::{header, StatusCode},
    response::IntoResponse,
};

pub async fn hello_world() -> &'static str {
    "Hello, bird!"
}

pub async fn seek() -> impl IntoResponse {
    (
        StatusCode::FOUND,
        [(
            header::LOCATION,
            "https://www.youtube.com/watch?v=9Gc4QTqslN4",
        )],
    )
}
