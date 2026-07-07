use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    Json,
};
use rusqlite::params;
use serde::Serialize;
use std::sync::Arc;

use crate::AppState;

fn human_size(bytes: i64) -> String {
    if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

#[derive(Serialize)]
pub struct BookDiagnostic {
    pub book_id: String,
    pub title: String,
    pub file_size_bytes: Option<i64>,
    pub file_size_human: String,
    pub range_header_received: Option<String>,
    pub range_forwarded_by_proxy: bool,
    pub via_cloudflare: bool,
    pub cf_ray: Option<String>,
    pub verdict: String,
}

pub async fn book_diagnostic(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<BookDiagnostic>, StatusCode> {
    let db = state.db.lock().await;
    let (title, file_size): (String, Option<i64>) = db
        .query_row(
            "SELECT title, file_size FROM books WHERE id = ?1",
            params![id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|_| StatusCode::NOT_FOUND)?;
    drop(db);

    let range_received = headers
        .get(header::RANGE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let range_forwarded = range_received.is_some();

    let cf_ray = headers
        .get("cf-ray")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let via_cloudflare = cf_ray.is_some();

    const LARGE_THRESHOLD: i64 = 15 * 1024 * 1024; // 15 MB

    let verdict = match (file_size, via_cloudflare, range_forwarded) {
        (Some(sz), true, false) if sz > LARGE_THRESHOLD => format!(
            "PROBLEM: file is {} and went through Cloudflare but Range header was NOT forwarded — \
             Zscaler will see the full response and likely drop it",
            human_size(sz)
        ),
        (Some(sz), true, true) if sz > LARGE_THRESHOLD => format!(
            "OK: file is {} and Range header WAS forwarded through Cloudflare — \
             chunked download should work",
            human_size(sz)
        ),
        (Some(sz), false, _) if sz > LARGE_THRESHOLD => format!(
            "PROBLEM: file is {} but request did not come through Cloudflare (no CF-Ray) — \
             direct connection, Range forwarding cannot be confirmed",
            human_size(sz)
        ),
        (Some(sz), _, _) => format!(
            "OK: file is {} — small enough to load without chunking",
            human_size(sz)
        ),
        (None, _, _) => "UNKNOWN: file_size not recorded in database for this book".to_string(),
    };

    Ok(Json(BookDiagnostic {
        book_id: id,
        title,
        file_size_bytes: file_size,
        file_size_human: file_size
            .map(human_size)
            .unwrap_or_else(|| "unknown".to_string()),
        range_header_received: range_received,
        range_forwarded_by_proxy: range_forwarded,
        via_cloudflare,
        cf_ray,
        verdict,
    }))
}
