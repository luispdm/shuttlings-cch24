use axum::{
    http::{header, StatusCode},
    response::IntoResponse,
};

pub async fn hello_bird() -> &'static str {
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
#[cfg(test)]
mod tests {
    use super::*;

    use axum::response::Response;

    #[tokio::test]
    async fn test_hello_bird() {
        let response = hello_bird().await;
        assert_eq!(response, "Hello, bird!");
    }

    #[tokio::test]
    async fn test_seek() {
        let response: Response = seek().await.into_response();
        assert_eq!(response.status(), StatusCode::FOUND);
        let headers = response.headers();
        assert_eq!(
            headers.get(header::LOCATION).unwrap(),
            "https://www.youtube.com/watch?v=9Gc4QTqslN4"
        );
    }
}
