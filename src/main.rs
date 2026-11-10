use std::io::Write;
use std::process::ExitCode;

use clap::Parser;
use ontodoc::cli::{Cli, Command, run};

fn main() -> ExitCode {
    let cli = Cli::parse();
    if matches!(cli.command, Command::Serve) {
        return serve(&cli.root);
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

fn serve(root: &std::path::Path) -> ExitCode {
    let result = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .and_then(|rt| rt.block_on(ontodoc::mcp::serve_stdio(root)));
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
