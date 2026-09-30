use axum::{
    extract::{Request, State},
    http::{HeaderValue, StatusCode, header},
    response::{Html, IntoResponse, Response},
};
use percent_encoding::percent_decode_str;
use std::io;
use std::path::{Component, Path, PathBuf};
use tower::ServiceExt;
use tower_http::services::ServeDir;

use crate::html::{DirEntry, generate_directory_listing};

#[derive(Clone)]
pub struct AppState {
    root: PathBuf,
    show_hidden: bool,
    serve_dir: ServeDir,
}

impl AppState {
    pub fn new(root: PathBuf, show_hidden: bool) -> Self {
        Self {
            serve_dir: ServeDir::new(&root),
            root,
            show_hidden,
        }
    }
}

pub async fn handle_request(State(state): State<AppState>, req: Request) -> Response {
    let Ok(decoded) = percent_decode_str(req.uri().path()).decode_utf8() else {
        return (StatusCode::BAD_REQUEST, "Invalid UTF-8 in path").into_response();
    };
    let Some(relative) = resolve_path(&decoded) else {
        return (StatusCode::FORBIDDEN, "Access denied").into_response();
    };

    let full_path = state.root.join(&relative);
    if !state.show_hidden
        && (is_hidden(&relative) || is_hidden_on_disk(&state.root, &full_path).await)
    {
        return StatusCode::NOT_FOUND.into_response();
    }

    // Directories without an index.html get a generated listing; ServeDir handles everything else.
    if is_dir(&full_path).await && !is_file(&full_path.join("index.html")).await {
        if !req.uri().path().ends_with('/') {
            return redirect_to_directory(&req);
        }
        let url_path = decoded.into_owned();
        return serve_directory_listing(full_path, url_path, state.show_hidden).await;
    }

    match state.serve_dir.oneshot(req).await {
        Ok(response) => response.into_response(),
        Err(infallible) => match infallible {},
    }
}

/// Turns a decoded URL path into a path relative to the served root.
///
/// Rejects anything that could step outside the root: `..` segments, and segments the platform
/// would parse as more than one plain component (such as `a\b` or `C:` on Windows).
fn resolve_path(decoded: &str) -> Option<PathBuf> {
    let mut relative = PathBuf::new();
    for segment in decoded.split('/') {
        match segment {
            "" | "." => {}
            ".." => return None,
            _ => {
                let mut components = Path::new(segment).components();
                match (components.next(), components.next()) {
                    (Some(Component::Normal(name)), None) => relative.push(name),
                    _ => return None,
                }
            }
        }
    }
    Some(relative)
}

/// True if any component of `relative` is a dotfile or dot-directory.
fn is_hidden(relative: &Path) -> bool {
    relative
        .components()
        .any(|c| c.as_os_str().as_encoded_bytes().starts_with(b"."))
}

/// Catches aliases that reach a hidden file under a different name: symlinks inside the root,
/// and Windows 8.3 short names (`GIT~1` for `.git`).
async fn is_hidden_on_disk(root: &Path, full_path: &Path) -> bool {
    match tokio::fs::canonicalize(full_path).await {
        Ok(real_path) => real_path.strip_prefix(root).is_ok_and(is_hidden),
        Err(_) => false,
    }
}

async fn is_dir(path: &Path) -> bool {
    tokio::fs::metadata(path).await.is_ok_and(|m| m.is_dir())
}

async fn is_file(path: &Path) -> bool {
    tokio::fs::metadata(path).await.is_ok_and(|m| m.is_file())
}

/// Sends `/dir` to `/dir/` (keeping the query string) so the listing's relative links resolve.
fn redirect_to_directory(req: &Request) -> Response {
    let uri = req.uri();
    let location = match uri.query() {
        Some(query) => format!("{}/?{}", uri.path(), query),
        None => format!("{}/", uri.path()),
    };
    match HeaderValue::try_from(location) {
        Ok(location) => (
            StatusCode::TEMPORARY_REDIRECT,
            [(header::LOCATION, location)],
        )
            .into_response(),
        Err(_) => StatusCode::BAD_REQUEST.into_response(),
    }
}

async fn serve_directory_listing(dir: PathBuf, url_path: String, show_hidden: bool) -> Response {
    let entries = tokio::task::spawn_blocking(move || read_entries(&dir, show_hidden))
        .await
        .unwrap_or_else(|err| Err(io::Error::other(err)));

    match entries {
        Ok(entries) => Html(generate_directory_listing(&url_path, &entries)).into_response(),
        Err(err) if err.kind() == io::ErrorKind::PermissionDenied => {
            (StatusCode::FORBIDDEN, "Permission denied").into_response()
        }
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to read directory: {err}"),
        )
            .into_response(),
    }
}

fn read_entries(dir: &Path, show_hidden: bool) -> io::Result<Vec<DirEntry>> {
    let mut entries = Vec::new();

    for entry in std::fs::read_dir(dir)?.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !show_hidden && name.starts_with('.') {
            continue;
        }

        // Follow symlinks so a link to a directory is listed as a directory;
        // fall back to the link itself when its target is missing.
        let metadata = std::fs::metadata(entry.path())
            .or_else(|_| entry.metadata())
            .ok();

        entries.push(DirEntry {
            name,
            is_dir: metadata.as_ref().is_some_and(|m| m.is_dir()),
            size: metadata.as_ref().map_or(0, |m| m.len()),
            modified: metadata
                .and_then(|m| m.modified().ok())
                .map(chrono::DateTime::from),
        });
    }

    // Directories first, then case-insensitive alphabetical
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_plain_paths() {
        assert_eq!(resolve_path("/"), Some(PathBuf::new()));
        assert_eq!(
            resolve_path("/a/b.txt"),
            Some(PathBuf::from("a").join("b.txt"))
        );
        assert_eq!(resolve_path("//a/./b/"), Some(PathBuf::from("a").join("b")));
        assert_eq!(resolve_path("/.../x"), Some(PathBuf::from("...").join("x")));
    }

    #[test]
    fn rejects_parent_segments() {
        assert_eq!(resolve_path("/../etc/passwd"), None);
        assert_eq!(resolve_path("/a/../../b"), None);
        assert_eq!(resolve_path("/a/.."), None);
    }

    #[cfg(windows)]
    #[test]
    fn rejects_windows_separators_and_prefixes() {
        assert_eq!(resolve_path(r"/..\..\Windows"), None);
        assert_eq!(resolve_path("/C:/Windows"), None);
        assert_eq!(resolve_path(r"/\\server\share"), None);
    }

    #[test]
    fn detects_hidden_components() {
        assert!(is_hidden(Path::new(".env")));
        assert!(is_hidden(&Path::new("a").join(".git").join("config")));
        assert!(!is_hidden(&Path::new("a").join("b.txt")));
        assert!(!is_hidden(Path::new("")));
    }
}
