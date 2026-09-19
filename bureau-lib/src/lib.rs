//! Bureau — governance framework for AI-assisted project work.
//! 
//! This crate contains the core domain logic for office management,
//! agent execution, sandbox enforcement, skills loading, and more.
//! 
//! # Architecture
//! 
//! - `config` — global + per-office configuration types
//! - `ticket` — ticket types and state machine lifecycle
//! - `sandbox` — permit-authorized file access enforcement
//! - `agent` — LLM agent loop with checkpointing protocol
//! - `skills` — SKILL.md discovery, parsing, and prompt assembly
//! - `run_config` — user-defined run configurations (`.bureau/run_configs.yaml`)
//! - `dashboard` — permit cards + ticket grid types (used by TUI crate)
//! - `research` — read-only explorer mode with deny-patterns
//! - `piping` — intent-setting + pipe-to-office flow

// Core domain modules
pub mod config;
pub mod ticket;
pub mod sandbox;
pub mod agent;
pub mod office;
pub mod provider;

// Skills system (P1)
pub mod skills;

// Run configurations (P2)
pub mod run_config;

// Dashboard types (P3 - used by tui binary crate)
pub mod dashboard;

// Research mode (read-only explorer)
pub mod research;

// Inspector permission system (P4) — ticket-bound scope + approval gate
pub mod inspector;

// Pipe-to-office flow
pub mod piping;

// Re-exports for convenience
pub use config::{GlobalConfig, OfficeConfig, OfficeEntry, ProviderConfig};
pub use ticket::{Ticket, TicketType, Phase};
pub use sandbox::Sandbox;
pub use skills::{SkillsLoader, Skill};
pub use agent::AgentLoop;
pub use office::Office;
pub use inspector::{TicketBoundScope, PermissionAction};
