use axum::{extract::Path, http::StatusCode, response::IntoResponse};

fn escape_string(s: &str) -> String {
    s.replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace("\"", "&quot;")
        .replace("'", "&#x27;")
        .replace("/", "&#x2F;")
}

pub async fn star() -> impl IntoResponse {
    (StatusCode::OK, "<div id=\"star\" class=\"lit\"></div>")
}

pub async fn present(Path(color): Path<String>) -> impl IntoResponse {
    let color = escape_string(&color);

    let (class, next) = match color {
        c if c == "blue" => ("blue", "purple"),
        c if c == "purple" => ("purple", "red"),
        c if c == "red" => ("red", "blue"),
        _ => return (StatusCode::IM_A_TEAPOT, "".to_string()),
    };

    let div = format!(
        "<div class=\"present {}\" hx-get=\"/23/present/{}\" hx-swap=\"outerHTML\"> <div class=\"ribbon\"></div><div class=\"ribbon\"></div><div class=\"ribbon\"></div><div class=\"ribbon\"></div></div>",
        class, next);

    (StatusCode::OK, div)
}

pub async fn ornament(Path((state, number)): Path<(String, String)>) -> impl IntoResponse {
    let (state, number) = (escape_string(&state), escape_string(&number));

    let (class, next_state) = match state {
        s if s == "on" => (" on", "off"),
        s if s == "off" => ("", "on"),
        _ => return (StatusCode::IM_A_TEAPOT, "".to_string()),
    };

    let div = format!(
        "<div class=\"ornament{}\" id=\"ornament{}\" hx-trigger=\"load delay:2s once\" hx-get=\"/23/ornament/{}/{}\" hx-swap=\"outerHTML\"></div>",
        class, number, next_state, number
    );

    (StatusCode::OK, div)
}
