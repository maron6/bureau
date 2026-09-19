//! Bureau TUI CLI — entry point for both interactive mode and CLI commands.
//! P7: run-config list/add + bureau start subcommands.
//! P8: new-office scaffold command.

mod app;

use anyhow::{Context, Result};
use std::path::PathBuf;
use bureau_lib::config::Mode;

/// Top-level command enumeration for the CLI.
enum Command {
    /// `bureau run-config list [mode]`
    ListConfigs { office_path: PathBuf, mode: Option<Mode> },
    /// `bureau run-config add --name X --mode execution ...`
    AddConfig { office_path: PathBuf, name: String, mode: Mode, description: Option<String>, skip_inspection: bool, permit_filter: Option<String> },
    /// `bureau start <office>`
    Start { office_path: PathBuf },
    /// `bureau new-office <path> --name X --language L`
    NewOffice { path: PathBuf, name: String, language: Option<String> },
    /// No-args / `bureau tui` — launch ratatui TUI
    Tui,
}

fn main() -> Result<()> {
    let cmd = parse_args()?;
    match &cmd {
        Command::ListConfigs { office_path, mode } => cmd_list_configs(office_path, mode.clone())?,
        Command::AddConfig { office_path, .. } => cmd_add_config(office_path, &cmd)?,
        Command::Start { office_path } => cmd_start(office_path)?,
        Command::NewOffice { path, .. } => cmd_new_office(path, &cmd)?,
        Command::Tui => cmd_tui()?,
    }
    Ok(())
}

// ====== P7: run-config list ======

fn cmd_list_configs(office_home: &PathBuf, mode: Option<Mode>) -> Result<()> {
    let configs = bureau_lib::run_config::load(&office_home).context("Failed to load run config")?;

    match mode {
        Some(m) => {
            let filtered: Vec<_> = configs.by_mode(m);
            if filtered.is_empty() {
                println!("No run configs found for mode '{}'.", m);
            } else {
                println!("Run configs (mode: {}):", m);
                for cfg in filtered {
                    println!(
                        "  - {} ({}) [skip_inspection={}]",
                        cfg.name,
                        cfg.description.as_deref().unwrap_or("(no description)"),
                        cfg.skip_inspection
                    );
                }
            }
        }
        None => {
            if configs.configs.is_empty() {
                println!("No run configs found. Use `bureau run-config add --name X --mode execution ...` to create one.");
            } else {
                println!("All run configs:");
                for cfg in &configs.configs {
                    println!(
                        "  - [{:<12}] {}",
                        cfg.run_mode,
                        cfg.name
                    );
                }
            }
        }
    }
    Ok(())
}

// ====== P7: run-config add ======

fn cmd_add_config(office_home: &PathBuf, cmd: &Command) -> Result<()> {
    let mut configs = bureau_lib::run_config::load(&office_home).unwrap_or_default();

    let (name, mode, description, skip_inspection, permit_filter) = match cmd {
        Command::AddConfig { name, mode, description, skip_inspection, permit_filter, .. } => {
            (name.clone(), mode.clone(), description.clone(), *skip_inspection, permit_filter.clone())
        }
        _ => anyhow::bail!("Wrong command variant"),
    };

    if configs.configs.iter().any(|c| c.name == name) {
        anyhow::bail!("A run config named '{}' already exists.", name);
    }

    let run_config = bureau_lib::run_config::RunConfig {
        name,
        run_mode: mode,
        description,
        skip_inspection,
        permit_filter,
        ..Default::default()
    };

    configs.configs.push(run_config);
    configs.save(&office_home).context("Failed to save run configs")?;
    println!("Run config added (mode: {}). Saved to .bureau/run_configs.yaml.", mode);
    Ok(())
}

// ====== P7: start ======

fn cmd_start(office_home: &PathBuf) -> Result<()> {
    let office_config = bureau_lib::config::load_office_config(&office_home).context("Failed to load office config")?;
    let run_configs = bureau_lib::run_config::load(&office_home).unwrap_or_default();
    let defaults = &run_configs.defaults;

    println!("Starting office '{}' in {} mode...", office_config.id, defaults.default);

    let by_mode: Vec<_> = run_configs.by_mode(defaults.default);
    if !by_mode.is_empty() {
        println!("Available configs for '{}':", defaults.default);
        for cfg in by_mode {
            println!("  - {} ({})", cfg.name, cfg.description.as_deref().unwrap_or("(no description)"));
        }
    } else {
        println!("No run configs for mode '{}'. Using defaults.", defaults.default);
    }

    if let Some(ref providers) = office_config.providers {
        if let Some(ref worker) = providers.worker {
            println!("  Worker provider: {} ({})", worker.name, worker.model);
        }
    }

    println!("\nOffice ready. Agent loop would start here.");
    Ok(())
}

// ====== P8: new-office scaffold ======

fn cmd_new_office(path: &PathBuf, cmd: &Command) -> Result<()> {
    let (name, language) = match cmd {
        Command::NewOffice { name, language, .. } => (name.clone(), language.clone()),
        _ => anyhow::bail!("Wrong command variant"),
    };

    // Create directory structure.
    let bureau_dir = path.join(".bureau");
    std::fs::create_dir_all(&bureau_dir).context("Failed to create .bureau directory")?;

    for d in &["tickets", "plans", "executions", "inspections", "archive"] {
        std::fs::create_dir_all(bureau_dir.join(d)).context(format!("Failed to create directory: {}", d))?;
    }

    // Generate config.yaml.
    let id = name.to_lowercase().replace(' ', "-");
    let content = format!(
        "# Bureau office configuration\nid: {}\nname: \"{}\"\nworksite: .\n",
        id,
        name,
    );
    std::fs::write(bureau_dir.join("config.yaml"), &content).context("Failed to write config.yaml")?;

    // Create empty run configs.
    let run_content = "defaults:\n  default: execution\nconfigs: []\n";
    std::fs::write(bureau_dir.join("run_configs.yaml"), run_content).context("Failed to write run_configs")?;

    // Register in global config.
    register_office_in_global(&name, &id, path)?;

    println!("Office '{}' scaffolded at {}. Created .bureau/ with default layout.", name, path.display());
    println!("Next steps:");
    println!("  1. Edit .bureau/config.yaml to set 'language' and other fields.");
    println!("  2. Run `cd {} && bureau-tui tui` or `bureau-tui start .`", path.display());
    Ok(())
}

/// Register the new office in ~/.bureau/config.yaml under offices list.
fn register_office_in_global(name: &str, id: &str, worksite: &PathBuf) -> Result<()> {
    let bureau_home = dirs::home_dir().unwrap_or_default().join(".bureau");
    let global_path = bureau_home.join("config.yaml");

    if bureau_home.exists() || std::fs::create_dir_all(&bureau_home).is_ok() {
        let worksite_str = worksite.canonicalize().unwrap_or_else(|_| PathBuf::from("."));
        let content = format!(
            "offices:\n  - id: \"{}\"\n    name: \"{}\"\n    worksite: {}\ndefault_provider:\n  name: openai\n  model: gpt-4o\n",
            id, name, worksite_str.display()
        );
        std::fs::write(&global_path, content).ok();
        println!("  Registered at {}", global_path.display());
    }
    Ok(())
}

// ====== TUI launcher ======

fn cmd_tui() -> Result<()> {
    // Load global config if available, otherwise scaffold.
    let global_config = match bureau_lib::config::load_global_config() {
        Ok(cfg) => cfg,
        Err(_) => {
            println!("No global config found. Run `bureau new-office <path>` first.");
            return Ok(());
        }
    };

    // In the full impl: create ratatui Terminal here and run TUI.
    // For now, print scaffolding info.
    let offices = global_config.offices;
    println!("TUI ready. Loaded {} office(s):", offices.len());
    for o in \u0026offices {
        println!("  - {} ({}): {}", o.id, o.name, o.worksite.display());
    }
    Ok(())
}

// ====== ARGUMENT PARSING (P7/P8) ======

fn parse_args() -> Result<Command> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        return Ok(Command::Tui);
    }

    match args[0].as_str() {
        "run-config" => {
            if args.len() < 2 {
                anyhow::bail!("`run-config` requires 'list' or 'add' as second arg");
            }
            match args[1].as_str() {
                "list" => {
                    let mode = if args.len() > 2 {
                        parse_mode(&args[2])
                    } else {
                        None
                    };
                    Ok(Command::ListConfigs { office_path: std::env::current_dir()?, mode })
                }
                "add" => {
                    // Parse flags from remaining args.
                    let name = flag_value(&args, "--name")
                        .ok_or_else(|| anyhow::anyhow!("--name required for `run-config add`"))?;
                    let mode_str = flag_value(&args, "--mode").ok_or_else(|| anyhow::anyhow!("--mode required for `run-config add`"))?;
                    let mode = parse_mode(&mode_str)
                        .ok_or_else(|| anyhow::anyhow!("Unknown mode '{}'", mode_str))?;
                    let description = flag_value(&args, "--description");
                    let skip_inspection = args.contains(&"--skip-inspection".to_string());
                    let permit_filter = flag_value(&args, "--permit-filter");
                    Ok(Command::AddConfig {
                        office_path: std::env::current_dir()?,
                        name, mode, description,
                        skip_inspection, permit_filter,
                    })
                }
                other => anyhow::bail!("Unknown run-config command '{}'. Use 'list' or 'add'.", other),
            }
        }
        "start" => {
            let office_path = args.get(1).map(|s| PathBuf::from(s)).unwrap_or_else(|| std::env::current_dir().unwrap());
            Ok(Command::Start { office_path })
        }
        "tui" => Ok(Command::Tui),
        "new-office" => {
            let path = args.get(1)
                .map(|s| PathBuf::from(s))
                .ok_or_else(|| anyhow::anyhow!("`new-office` requires a directory path"))?;
            let name = flag_value(&args, "--name").ok_or_else(|| anyhow::anyhow!("--name required"))?;
            let language = flag_value(&args, "--language");
            Ok(Command::NewOffice { path, name, language })
        }
        other => anyhow::bail!("Unknown command '{}'. Available: run-config, start, new-office, tui", other),
    }
}

/// Parse a mode string to Mode enum variant.
fn parse_mode(s: &str) -> Option<Mode> {
    match s.to_lowercase().as_str() {
        "interviewing" => Some(Mode::Interviewing),
        "planning" => Some(Mode::Planning),
        "execution" => Some(Mode::Execution),
        "inspection" => Some(Mode::Inspection),
        "archival" => Some(Mode::Archival),
        _ => None,
    }
}

/// Extract a flag value from args list: `flag_value(&args, "--foo")` → value after --foo.
fn flag_value(args: &[String], flag: &str) -> Option<String> {
    for i in 0..args.len() {
        if args[i] == flag && i + 1 < args.len() {
            return Some(args[i + 1].clone());
        }
        // Also handle --flag=value syntax.
        if let Some(val) = args[i].strip_prefix(flag) {
            if val.starts_with('=') && val.len() > 1 {
                return Some(val[1..].to_string());
            }
        }
    }
    None
}
