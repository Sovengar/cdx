mod cli;
mod config;
mod tui;
mod walker;
mod preview;
mod search;
mod zoxide;

use std::path::Path;

use clap::Parser;
use cli::Cli;

fn main() -> anyhow::Result<()> {
    config::init();
    let cli = Cli::parse();
    let query = cli.query.join(" ");

    if cli.grep {
        return search::global_search(&query);
    }

    if query == "~" || query == "..." {
        let home = dirs::home_dir().unwrap();
        emit_result(Some(&home));
        return Ok(());
    }

    if !query.is_empty() {
        let path = std::path::PathBuf::from(&query);
        if path.is_dir() {
            emit_result(Some(&path));
            return Ok(());
        }
        if let Some(z_path) = zoxide::query(&query) {
            emit_result(Some(&z_path));
            return Ok(());
        }
    }

    let initial_query = if query.is_empty() { None } else { Some(query) };
    let exit_path = tui::run(initial_query)?;
    emit_result(exit_path.as_deref());
    Ok(())
}

fn emit_result(path: Option<&Path>) {
    if let Some(p) = path {
        let result_file = std::env::temp_dir().join("cdx-rs-result.txt");
        let _ = std::fs::write(&result_file, format!("{}\n", p.display()));
        println!("{}", p.display());
    }
}
