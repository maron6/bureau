//! Pipe-to-office workflow — user's choice when piping research results to an office.
//! Follows Q32/B: FORCES intent-setting (what should the bureaucrant focus on?) at 
//! pipe time, preventing silent data dumping into bureaucracy.
//! Follows Q27/B+C: two modes — trigger new bureacracy from research OR save to existing office.

use std::path::PathBuf;

// ==========================================================================
// Pipe Target Selection
// User's choice when piping research results (Q27): 
// "trigger bureau governance" (new office) or "save to existing" (existing office)
// Follows Q32/B: BOTH require focus_intent — intent-setting cannot be skipped.
// ==========================================================================

/// What the user selects in the pipe-to-office dialog (rendered in TUI).  
/// Drives both destination AND whether bureaucracy should start from this content.
#[derive(Debug, Clone)]
pub enum PipeTarget {
    /// Save research findings into an existing office without triggering bureau.
    ExistingOffice {
        /// The target office id (from global config registry) where notes are saved.
        office_id: String,
        /// Where in the office's .bureau/ dir to save content (e.g., "interview_prework").
        target_location: PathBuf,
        // Note: for existing-office pipe, user may still trigger bureaucrant on this content (Q27).
        trigger_bureaucrat_when_saving: bool,  // whether bureau governance starts from this save
    },

    /// Create a new office pipeline starting from this research context.
    NewOffice {
        /// Name of the new office to create (user sets or auto-generated). 
        name: String,
        /// Intent for what the future bureacrat should focus on in this new office (Q32/B)  
        /// This is FORCED — user cannot pipe without setting bureau intent.
        focus_intent: String,
        /// Initial scope/requirement derived from research findings to feed bureaucrant.
        pub initial_scope: Option<String>,
    },
}

/// The complete flow data for a pipe-to-office action (Q32/B intent-setting UX).
#[derive(Debug, Clone)]
pub struct PipelineChoice {
    /// Which target the user selected (existing vs new office).
    pub target: PipeTarget,
    
    // Note: `trigger_bureaucrat when_saving` is required for ExistingOffice if
    // Q32/B forces intent-setting — then focus_intent is also required here.
}

/// UX context for the pipe dialog in TUI.
#[derive(Debug, Clone)]
pub struct PipeUX {
    /// The source research session being piped from.
    pub source_session_id: Option<String>, 

   /// All available targets the user can choose (existing offices + "create new").
    pub existing_offices: Vec<String>,

    // Which target did the user select? (filled in after pipe dialog completes)
    pub selected_target: Option<PipeTarget>,
}

