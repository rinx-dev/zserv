use clap::Parser;
use std::net::IpAddr;
use std::path::PathBuf;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about = "zserv: A simple, lightweight HTTP file server", long_about = None)]
pub struct Config {
    /// Port to listen on (0 picks a free port)
    #[arg(short, long, default_value_t = 8080)]
    pub port: u16,

    /// Address to bind to
    #[arg(short, long, default_value = "0.0.0.0")]
    pub address: IpAddr,

    /// Directory to serve
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Enable CORS headers
    #[arg(long, default_value_t = false)]
    pub cors: bool,

    /// List and serve hidden files (names starting with '.')
    #[arg(long, default_value_t = false)]
    pub hidden: bool,

    /// Suppress log output
    #[arg(short, long, default_value_t = false)]
    pub silent: bool,
}
