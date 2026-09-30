use chrono::{DateTime, Local};
use humansize::{DECIMAL, format_size};
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};
use std::fmt::Write;

/// Everything except RFC 3986 "unreserved" characters gets percent-encoded, so a file name
/// can never be read as a query (`?`), fragment (`#`), escape (`%`) or HTML attribute break (`"`).
const PATH_SEGMENT: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'_')
    .remove(b'.')
    .remove(b'~');

const STYLE: &str = "\
:root { color-scheme: light dark; --fg: #1f2328; --muted: #59636e; --link: #0969da; --line: #d1d9e0; --head: #f6f8fa; }
@media (prefers-color-scheme: dark) { :root { --fg: #e6edf3; --muted: #9198a1; --link: #4493f8; --line: #3d444d; --head: #151b23; } }
body { margin: 0; padding: 20px; font-family: -apple-system, BlinkMacSystemFont, \"Segoe UI\", Roboto, Helvetica, Arial, sans-serif; line-height: 1.5; color: var(--fg); background: Canvas; }
h1 { font-size: 1.4em; font-weight: 600; overflow-wrap: anywhere; }
table { width: 100%; border-collapse: collapse; }
th, td { text-align: left; padding: 8px; border-bottom: 1px solid var(--line); }
th { background: var(--head); }
td.name { overflow-wrap: anywhere; }
td.size, th.size { text-align: right; font-variant-numeric: tabular-nums; white-space: nowrap; }
td.date { color: var(--muted); font-variant-numeric: tabular-nums; white-space: nowrap; }
a { text-decoration: none; color: var(--link); }
a:hover { text-decoration: underline; }
.icon { margin-right: 5px; }
footer { color: var(--muted); }
@media (max-width: 600px) { .date { display: none; } }
";

pub struct DirEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: Option<DateTime<Local>>,
}

/// Renders the listing for the directory at `url_path`, the decoded request path (e.g. `/My Photos/`).
///
/// Links are relative, so the page must be served from a URL that ends in `/`.
pub fn generate_directory_listing(url_path: &str, entries: &[DirEntry]) -> String {
    let title = escape_html(url_path);
    let mut html = String::new();

    let _ = write!(
        html,
        "<!DOCTYPE html><html><head><meta charset=\"utf-8\">\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\
         <title>Index of {title}</title><style>{STYLE}</style></head><body>\
         <h1>Index of {title}</h1>"
    );

    if url_path != "/" {
        html.push_str("<p><a href=\"../\">⬅ Parent Directory</a></p>");
    }

    html.push_str(
        "<table><thead><tr><th>Name</th><th class=\"size\">Size</th>\
         <th class=\"date\">Last Modified</th></tr></thead><tbody>",
    );

    for entry in entries {
        let (icon, suffix, size) = if entry.is_dir {
            ("📁", "/", "-".to_string())
        } else {
            ("📄", "", format_size(entry.size, DECIMAL))
        };
        let modified = entry.modified.map_or_else(
            || "-".to_string(),
            |dt| dt.format("%Y-%m-%d %H:%M:%S").to_string(),
        );

        let _ = write!(
            html,
            "<tr><td class=\"name\"><span class=\"icon\">{icon}</span><a href=\"{href}{suffix}\">{name}{suffix}</a></td>\
             <td class=\"size\">{size}</td><td class=\"date\">{modified}</td></tr>",
            href = utf8_percent_encode(&entry.name, PATH_SEGMENT),
            name = escape_html(&entry.name),
        );
    }

    let _ = write!(
        html,
        "</tbody></table><hr><footer><em>zserv {}</em></footer></body></html>",
        env!("CARGO_PKG_VERSION")
    );

    html
}

fn escape_html(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(c),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(name: &str) -> DirEntry {
        DirEntry {
            name: name.to_string(),
            is_dir: false,
            size: 1,
            modified: None,
        }
    }

    #[test]
    fn escapes_html_special_characters() {
        assert_eq!(
            escape_html(r#"<img src=x onerror="alert('x')">&"#),
            "&lt;img src=x onerror=&quot;alert(&#39;x&#39;)&quot;&gt;&amp;"
        );
    }

    #[test]
    fn file_names_cannot_inject_markup() {
        let html = generate_directory_listing("/", &[file("<img src=x onerror=alert(1)>.txt")]);
        assert!(!html.contains("<img"));
        assert!(html.contains("&lt;img src=x onerror=alert(1)&gt;.txt"));
    }

    #[test]
    fn links_percent_encode_reserved_characters() {
        let html = generate_directory_listing(
            "/",
            &[
                file("a#b.txt"),
                file("what?.txt"),
                file("%41.txt"),
                file("say \"hi\".txt"),
            ],
        );
        assert!(html.contains(r#"href="a%23b.txt""#));
        assert!(html.contains(r#"href="what%3F.txt""#));
        assert!(html.contains(r#"href="%2541.txt""#));
        assert!(html.contains(r#"href="say%20%22hi%22.txt""#));
    }

    #[test]
    fn directories_link_with_trailing_slash() {
        let dir = DirEntry {
            is_dir: true,
            ..file("写真")
        };
        let html = generate_directory_listing("/", &[dir]);
        assert!(html.contains(r#"href="%E5%86%99%E7%9C%9F/">写真/</a>"#));
    }

    #[test]
    fn heading_shows_decoded_escaped_path() {
        let html = generate_directory_listing("/My Photos/<b>/", &[]);
        assert!(html.contains("<h1>Index of /My Photos/&lt;b&gt;/</h1>"));
        assert!(html.contains(r#"<a href="../">"#));
    }

    #[test]
    fn root_has_no_parent_link() {
        assert!(!generate_directory_listing("/", &[]).contains("Parent Directory"));
    }
}
