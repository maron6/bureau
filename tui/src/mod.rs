/// Ratatui TUI module for bureau — sidebar + command palette navigation.
//! Follows Decision Q15/A: Sidebar showing all offices as status cards.
//! Follows Decision Q15/C: Command palette (Ctrl+P-style) for fast switching.

mod app;

pub use app::App;

use bureau_lib::config::GlobalConfig;
use anyhow::Result;
use crossterm::{event::{DisableMouseCapture, EnableMouseCapture, Event as CrosstermEvent, KeyCode, KeyEventKind, key::KeyModifiers}, execute, terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen}};
use ratatui::{backend::CrosstermBackend, Terminal};

/// The main event loop for the TUI: processes events and renders UI.
pub async fn run(terminal: &mut ratatui::Terminal<CrosstermBackend<std::io::Stdout>>, app: &mut App) -> Result<()> {
    // Enter alternate screen mode for full-terminal TUI experience.
    enable_raw_mode()?;
    execute!(std::io::stdout(), EnterAlternateScreen, EnableMouseCapture)?;

    let mut terminal = ratatui::Terminal::new(CrosstermBackend::new(std::io::stdout()))?;
    
    // TODO: implement the event loop, rendering cycle, and UI state management here.
    // This is scaffolding for the ratatui layer (Decision Q15/A+C).
    
    disable_raw_mode()?;
    execute!(std::io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
    
    Ok(())
}

// The TUI needs to be configured based on user prefs from global config.
impl bureau_lib::config::TuiLayout {
    pub fn is_sidebar(&self) -> bool { unimplemented!() }
}

