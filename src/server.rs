use axum::{
    Router,
    http::{Extensions, HeaderMap, HeaderValue, Method, StatusCode, Version, header},
    middleware,
    routing::get,
};
use std::path::PathBuf;
use std::time::Duration;
use tower_http::{
    compression::{CompressionLayer, Predicate, predicate::DefaultPredicate},
    cors::{Any, CorsLayer},
    set_header::SetResponseHeaderLayer,
    timeout::TimeoutLayer,
};

use crate::cli::Config;
use crate::handlers::{AppState, handle_request};
use crate::logging::print_request_log;

/// How long a request may take to start its response before zserv answers `408 Request Timeout`.
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

/// Builds the app for an already-canonicalized `root` directory.
pub fn build_app(config: &Config, root: PathBuf) -> Router {
    let mut router = Router::new()
        // Every URL is a file or directory lookup. GET also answers HEAD; other methods get 405.
        .fallback(get(handle_request))
        .with_state(AppState::new(root, config.hidden))
        .layer(CompressionLayer::new().compress_when(DefaultPredicate::new().and(is_compressible)))
        // Always revalidate, so edited files show up on the next reload.
        .layer(SetResponseHeaderLayer::overriding(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-cache"),
        ))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            REQUEST_TIMEOUT,
        ));

    if config.cors {
        let cors = CorsLayer::new()
            .allow_methods([Method::GET, Method::HEAD, Method::OPTIONS])
            .allow_origin(Any);
        router = router.layer(cors);
    }

    // Outermost, so the log shows the final status, including timeouts and CORS preflights.
    if !config.silent {
        router = router.layer(middleware::from_fn(print_request_log));
    }

    router
}

/// Compress only text-like responses. Media and archives are already compressed, and compressing
/// them anyway drops `Content-Length`, so browsers can't show download progress.
fn is_compressible(_: StatusCode, _: Version, headers: &HeaderMap, _: &Extensions) -> bool {
    let Some(content_type) = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
    else {
        return false;
    };
    let essence = content_type.split(';').next().unwrap_or_default().trim();

    essence.starts_with("text/")
        || essence.ends_with("+json")
        || essence.ends_with("+xml")
        || matches!(
            essence,
            "application/json"
                | "application/xml"
                | "application/javascript"
                | "application/x-javascript"
                | "application/wasm"
                | "font/ttf"
                | "font/otf"
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use axum::response::Response;
    use clap::Parser;
    use std::fs;
    use std::path::Path;
    use tower::ServiceExt;

    fn fixture() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::write(root.join("hello.txt"), "hello world").unwrap();
        fs::write(root.join(".env"), "SECRET=1").unwrap();
        fs::create_dir(root.join(".git")).unwrap();
        fs::write(root.join(".git").join("config"), "[core]").unwrap();
        fs::create_dir(root.join("sub")).unwrap();
        fs::write(root.join("sub").join("inner.txt"), "inner").unwrap();
        fs::create_dir(root.join("site")).unwrap();
        fs::write(root.join("site").join("index.html"), "<h1>home</h1>").unwrap();
        dir
    }

    fn app(root: &Path, flags: &[&str]) -> Router {
        let args = ["zserv", "--silent"]
            .into_iter()
            .chain(flags.iter().copied())
            .chain([root.to_str().unwrap()]);
        build_app(&Config::parse_from(args), root.canonicalize().unwrap())
    }

    async fn send(app: &Router, method: Method, uri: &str, headers: &[(&str, &str)]) -> Response {
        let mut req = Request::builder().method(method).uri(uri);
        for (name, value) in headers {
            req = req.header(*name, *value);
        }
        app.clone()
            .oneshot(req.body(Body::empty()).unwrap())
            .await
            .unwrap()
    }

    async fn get(app: &Router, uri: &str) -> Response {
        send(app, Method::GET, uri, &[]).await
    }

    async fn text(res: Response) -> String {
        let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    fn header(res: &Response, name: header::HeaderName) -> Option<&str> {
        res.headers().get(name).map(|v| v.to_str().unwrap())
    }

    /// Every `href` in a listing, in order.
    fn hrefs(html: &str) -> Vec<String> {
        html.split("href=\"")
            .skip(1)
            .map(|rest| rest[..rest.find('"').unwrap()].to_string())
            .collect()
    }

    #[tokio::test]
    async fn every_listing_link_resolves_to_its_file() {
        let dir = tempfile::tempdir().unwrap();
        let mut names = vec![
            "a#b.txt",
            "%41.txt",
            "50% off.txt",
            "a&b.txt",
            "space name.txt",
            "写真.txt",
            "semi;colon+plus.txt",
        ];
        if cfg!(unix) {
            // Not valid file names on Windows
            names.extend(["what?.txt", "say \"hi\".txt", "back\\slash.txt", "<b>.txt"]);
        }
        for name in &names {
            fs::write(dir.path().join(name), name).unwrap();
        }

        let app = app(dir.path(), &[]);
        let links = hrefs(&text(get(&app, "/").await).await);
        assert_eq!(links.len(), names.len());

        for link in links {
            let res = get(&app, &format!("/{link}")).await;
            assert_eq!(res.status(), StatusCode::OK, "link {link} is broken");
            assert!(names.contains(&text(res).await.as_str()));
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn file_names_cannot_inject_markup() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("<img src=x onerror=alert(1)>.txt"), "x").unwrap();

        let html = text(get(&app(dir.path(), &[]), "/").await).await;
        assert!(!html.contains("<img"));
        assert!(html.contains("&lt;img src=x onerror=alert(1)&gt;.txt"));
    }

    #[tokio::test]
    async fn listing_heading_shows_decoded_path() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("My Photos")).unwrap();

        let html = text(get(&app(dir.path(), &[]), "/My%20Photos/").await).await;
        assert!(html.contains("<h1>Index of /My Photos/</h1>"));
    }

    #[tokio::test]
    async fn hidden_files_are_not_listed_or_served() {
        let dir = fixture();
        let app = app(dir.path(), &[]);

        let html = text(get(&app, "/").await).await;
        assert!(!html.contains(".env") && !html.contains(".git"));
        assert!(html.contains("hello.txt"));

        for uri in ["/.env", "/%2Eenv", "/.git/config", "/.git/", "/sub/../.env"] {
            let status = get(&app, uri).await.status();
            assert!(status.is_client_error(), "{uri} returned {status}");
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn hidden_files_are_not_served_through_symlinks() {
        let dir = fixture();
        std::os::unix::fs::symlink(".git", dir.path().join("gitlink")).unwrap();

        let res = get(&app(dir.path(), &[]), "/gitlink/config").await;
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn hidden_flag_lists_and_serves_dotfiles() {
        let dir = fixture();
        let app = app(dir.path(), &["--hidden"]);

        assert!(text(get(&app, "/").await).await.contains(".env"));
        assert_eq!(text(get(&app, "/.env").await).await, "SECRET=1");
    }

    #[tokio::test]
    async fn parent_segments_are_rejected() {
        let dir = fixture();
        let app = app(dir.path(), &[]);

        for uri in [
            "/../etc/passwd",
            "/sub/%2e%2e/%2e%2e/etc/passwd",
            "/sub/..%2f..%2fetc",
        ] {
            assert_eq!(
                get(&app, uri).await.status(),
                StatusCode::FORBIDDEN,
                "{uri}"
            );
        }
    }

    #[tokio::test]
    async fn directories_redirect_to_trailing_slash() {
        let dir = fixture();
        let app = app(dir.path(), &[]);

        let res = get(&app, "/sub?sort=name").await;
        assert_eq!(res.status(), StatusCode::TEMPORARY_REDIRECT);
        assert_eq!(header(&res, header::LOCATION), Some("/sub/?sort=name"));

        let html = text(get(&app, "/sub/").await).await;
        assert!(html.contains(r#"href="inner.txt""#) && html.contains(r#"href="../""#));
    }

    #[tokio::test]
    async fn index_html_is_served_for_directories() {
        let dir = fixture();
        let res = get(&app(dir.path(), &[]), "/site/").await;
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(text(res).await, "<h1>home</h1>");
    }

    #[tokio::test]
    async fn responses_are_always_revalidated() {
        let dir = fixture();
        let app = app(dir.path(), &[]);

        for uri in ["/hello.txt", "/"] {
            let res = get(&app, uri).await;
            assert_eq!(
                header(&res, header::CACHE_CONTROL),
                Some("no-cache"),
                "{uri}"
            );
        }
    }

    #[tokio::test]
    async fn compresses_text_but_not_archives() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("notes.txt"), "lorem ipsum ".repeat(100)).unwrap();
        fs::write(dir.path().join("photos.zip"), vec![7u8; 1200]).unwrap();
        let app = app(dir.path(), &[]);
        let gzip = [("accept-encoding", "gzip")];

        let res = send(&app, Method::GET, "/notes.txt", &gzip).await;
        assert_eq!(header(&res, header::CONTENT_ENCODING), Some("gzip"));

        let res = send(&app, Method::GET, "/photos.zip", &gzip).await;
        assert_eq!(header(&res, header::CONTENT_ENCODING), None);
        assert_eq!(header(&res, header::CONTENT_LENGTH), Some("1200"));
    }

    #[tokio::test]
    async fn only_get_and_head_are_allowed() {
        let dir = fixture();
        let app = app(dir.path(), &[]);

        for uri in ["/", "/hello.txt"] {
            let status = send(&app, Method::POST, uri, &[]).await.status();
            assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED, "{uri}");
        }

        let res = send(&app, Method::HEAD, "/hello.txt", &[]).await;
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(text(res).await, "");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn symlinked_directories_are_listed_as_directories() {
        let dir = fixture();
        std::os::unix::fs::symlink("sub", dir.path().join("link")).unwrap();

        let html = text(get(&app(dir.path(), &[]), "/").await).await;
        assert!(html.contains(r#"href="link/""#));
    }

    #[tokio::test]
    async fn cors_flag_adds_headers() {
        let dir = fixture();
        let origin = [("origin", "http://example.com")];

        let res = send(
            &app(dir.path(), &["--cors"]),
            Method::GET,
            "/hello.txt",
            &origin,
        )
        .await;
        assert_eq!(header(&res, header::ACCESS_CONTROL_ALLOW_ORIGIN), Some("*"));

        let res = send(&app(dir.path(), &[]), Method::GET, "/hello.txt", &origin).await;
        assert_eq!(header(&res, header::ACCESS_CONTROL_ALLOW_ORIGIN), None);
    }
}
