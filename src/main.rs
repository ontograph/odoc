use std::io::Write;
use std::process::ExitCode;

use clap::Parser;
use ontodoc::cli::{Cli, Command, run};

fn main() -> ExitCode {
    let cli = Cli::parse();
    if let Command::Serve { auto_index } = cli.command {
        return serve(&cli.root, auto_index);
    }
    if let Command::ServeHttp { docs_share, listen } = &cli.command {
        return serve_http(docs_share, *listen);
    }
    match run(cli) {
        Ok(json) => match writeln!(std::io::stdout().lock(), "{json}") {
            Ok(()) => ExitCode::SUCCESS,
            Err(_) => ExitCode::FAILURE,
        },
        Err(err) => {
            eprintln!("{}", err.to_json());
            ExitCode::FAILURE
        }
    }
}

fn serve(root: &std::path::Path, auto_index: bool) -> ExitCode {
    let result = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .and_then(|rt| rt.block_on(ontodoc::mcp::serve_stdio(root, auto_index)));
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!(
                "{}",
                serde_json::json!({ "error": "serve", "message": err.to_string() })
            );
            ExitCode::FAILURE
        }
    }
}

fn serve_http(docs_share: &std::path::Path, listen: std::net::SocketAddr) -> ExitCode {
    let result = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(ontodoc::http::HttpError::from)
        .and_then(|rt| {
            rt.block_on(async {
                let shutdown = tokio_util::sync::CancellationToken::new();
                let signal = shutdown.clone();
                let mut term =
                    tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
                tokio::spawn(async move {
                    tokio::select! {
                        _ = tokio::signal::ctrl_c() => {}
                        _ = term.recv() => {}
                    }
                    signal.cancel();
                });
                ontodoc::http::serve(docs_share, listen, shutdown).await
            })
        });
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!(
                "{}",
                serde_json::json!({ "error": "serve_http", "message": err.to_string() })
            );
            ExitCode::FAILURE
        }
    }
}
