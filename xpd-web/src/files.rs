use crate::{error::HttpError, AppState};
use axum::{extract::State, response::Html};

#[allow(clippy::unused_async)]
pub async fn serve_index(State(state): State<AppState>) -> Result<Html<String>, HttpError> {
    let mut context = tera::Context::new();
    context.insert("root_url", &state.root_url);
    Ok(Html(
        state
            .tera
            .render("index.html", &context)
            .map_err(|e| HttpError::new(e.into(), state))?,
    ))
}

#[allow(clippy::unused_async)]
pub async fn serve_privacy(State(state): State<AppState>) -> Result<Html<String>, HttpError> {
    let mut context = tera::Context::new();
    context.insert("root_url", &state.root_url);
    Ok(Html(
        state
            .tera
            .render("privacy.html", &context)
            .map_err(|e| HttpError::new(e.into(), state))?,
    ))
}

#[allow(clippy::unused_async)]
pub async fn serve_terms(State(state): State<AppState>) -> Result<Html<String>, HttpError> {
    let mut context = tera::Context::new();
    context.insert("root_url", &state.root_url);
    Ok(Html(
        state
            .tera
            .render("terms.html", &context)
            .map_err(|e| HttpError::new(e.into(), state))?,
    ))
}

#[allow(clippy::unused_async)]
pub async fn serve_404(State(state): State<AppState>) -> Result<Html<String>, HttpError> {
    let mut context = tera::Context::new();
    context.insert("root_url", &state.root_url);
    Ok(Html(
        state
            .tera
            .render("404.html", &context)
            .map_err(|e| HttpError::new(e.into(), state))?,
    ))
}

#[allow(clippy::unused_async)]
pub async fn serve_robots(State(state): State<AppState>) -> Result<String, HttpError> {
    let mut context = tera::Context::new();
    context.insert("root_url", &state.root_url);
    state
        .tera
        .render("robots.txt", &context)
        .map_err(|e| HttpError::new(e.into(), state))
}

#[allow(clippy::unused_async)]
pub async fn serve_sitemap(State(state): State<AppState>) -> Result<String, HttpError> {
    let mut context = tera::Context::new();
    context.insert("root_url", &state.root_url);
    state
        .tera
        .render("sitemap.txt", &context)
        .map_err(|e| HttpError::new(e.into(), state))
}

#[allow(clippy::unused_async)]
pub async fn serve_css() -> ([(&'static str, &'static str); 1], &'static [u8]) {
    (
        [("Content-Type", "text/css")],
        include_bytes!("resources/main.css").as_slice(),
    )
}

#[allow(clippy::unused_async)]
pub async fn serve_font() -> ([(&'static str, &'static str); 2], &'static [u8]) {
    (
        [
            ("Content-Type", "font/woff2"),
            ("Cache-Control", "max-age=86400"),
        ],
        include_bytes!("resources/MontserratAlt1.woff2").as_slice(),
    )
}

#[allow(clippy::unused_async)]
pub async fn serve_icon() -> ([(&'static str, &'static str); 2], &'static [u8]) {
    (
        [
            ("Content-Type", "image/png"),
            ("Cache-Control", "max-age=86400"),
        ],
        include_bytes!("resources/favicon.png").as_slice(),
    )
}
