use axum::{extract::Request, http::header, middleware::Next, response::Response};
use chrono::Utc;
use colored::Colorize;
use std::time::Instant;

pub async fn print_request_log(req: Request, next: Next) -> Response {
    let method = req.method().clone();
    let uri = req.uri().clone();

    // Extract User-Agent header safely
    let user_agent = req
        .headers()
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-")
        .to_owned();

    let started = Instant::now();
    let response = next.run(req).await;
    let elapsed = started.elapsed();

    let status = response.status();
    let code = status.as_str();
    let code = if status.is_server_error() {
        code.red()
    } else if status.is_client_error() {
        code.yellow()
    } else if status.is_redirection() {
        code.cyan()
    } else {
        code.green()
    };

    // Format ISO8601 time with milliseconds (e.g., 2026-02-16T06:08:54.014Z)
    let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ");

    // [TIMESTAMP]  "METHOD URI" STATUS DURATION "UA"
    println!(
        "[{}]  \"{} {}\" {} {} \"{}\"",
        timestamp.to_string().dimmed(),
        method.as_str().cyan(),
        uri.to_string().cyan(),
        code,
        format!("{elapsed:.1?}").dimmed(),
        user_agent.dimmed()
    );

    response
}
