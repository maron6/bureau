//! # Bureau
//! A Rust TUI application for governing AI-assisted project work through a structured 
//! bureau of modes: bureaucrant (interview), architect (planning), workers (execution), 
//! inspector (quality review), archivist (documentation), and researchers (exploration).

mod agent;
mod config;
mod office;
mod piping;  // Q27/B+C: pipe-to-office workflow with intent-setting (Q32)
mod provider; 
mod research;  // Q26/C: sandbox-unrestricted read-only exploration mode
mod sandbox;
mod ticket;
mod tui;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let global_config = config::load_global_config()?;
    
    // Initialize ratatui terminal with Crossterm backend.
    let _term = ratatui::Terminal::new(
        ratatui::backend::CrosstermBackend::new(std::io::stdout())
    )?;

    // TODO: create TUI app instance, run event loop (with sidebar + command palette)
    let mut app = tui::App::new(global_config);
    let _result = tui::run(&mut term, &mut app).await;

    Ok(())
}

