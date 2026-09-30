use crate::cli::Config;
use crate::server::REQUEST_TIMEOUT;
use colored::Colorize;
use local_ip_address::list_afinet_netifas;
use std::net::SocketAddr;
use std::path::Path;

pub fn print_banner(config: &Config, root: &Path, addr: SocketAddr) {
    let app_name = env!("CARGO_PKG_NAME").yellow();
    let version = env!("CARGO_PKG_VERSION");

    // canonicalize() returns verbatim paths (\\?\C:\...) on Windows; show the familiar form
    let root = root.to_string_lossy();
    let display_path = root.strip_prefix(r"\\?\").unwrap_or(&root);

    println!("Starting up {}, serving {}", app_name, display_path.cyan());
    println!();
    println!("{} version: {}", app_name, version.bright_white());
    println!();
    println!("{} settings:", app_name);
    println!(
        "CORS: {}",
        if config.cors {
            "enabled".green()
        } else {
            "disabled".red()
        }
    );
    println!(
        "Hidden Files: {}",
        if config.hidden {
            "visible".yellow()
        } else {
            "hidden".green()
        }
    );
    println!("Cache: {}", "disabled".red());
    println!(
        "Compression: {}",
        "gzip, br, zstd (text files)".bright_white()
    );
    println!(
        "Request Timeout: {}",
        format!("{} seconds", REQUEST_TIMEOUT.as_secs()).bright_white()
    );
    println!("Directory Listings: {}", "visible".green());
    println!();
    println!("Available on:");

    // If bound to 0.0.0.0 or ::, list all available interfaces
    if addr.ip().is_unspecified() {
        if let Ok(interfaces) = list_afinet_netifas() {
            for (_, ip) in interfaces {
                // Filter by family (IPv4 vs IPv6) based on binding
                if addr.is_ipv4() && ip.is_ipv4() {
                    let url = format!("http://{}:{}", ip, addr.port());
                    println!("  {}", url.underline().blue());
                } else if addr.is_ipv6() && ip.is_ipv6() {
                    let url = format!("http://[{}]:{}", ip, addr.port());
                    println!("  {}", url.underline().blue());
                }
            }
        } else {
            // Fallback
            if addr.is_ipv4() {
                let url = format!("http://127.0.0.1:{}", addr.port());
                println!("  {}", url.underline().blue());
            } else {
                let url = format!("http://[::1]:{}", addr.port());
                println!("  {}", url.underline().blue());
            }
        }
    } else {
        // Just print the bound address
        let url = format!("http://{}", addr);
        println!("  {}", url.underline().blue());
    }

    println!("Hit CTRL-C to stop the server");
    println!();
}
