//! skry — see every server, install nothing.

mod cli;

use std::process::ExitCode;

use clap::Parser;

#[tokio::main]
async fn main() -> ExitCode {
    // The ring provider is compiled in; make it the process default so
    // every TLS client (webhooks, certificate checks) agrees.
    let _ = rustls::crypto::ring::default_provider().install_default();
    let cli = cli::Cli::parse();
    match cli::run(cli).await {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::from(1)
        }
    }
}
