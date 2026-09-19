//! Dashboard types — P3 widget rendering order: cards→grid→workers/permissions.
//! Full UX spec lives in D22 per ADR 0001 + grilling Q49/Q37 decisions.

use serde::{Deserialize, Serialize};

// Permit card displayed at top of active view (ADR 0001 D22) 
#[derive(Debug, Clone, Deserialize, Serialize)]   
pub struct PermitCard {
    pub id: String,
   pub title: String,
   pub status: String,  
}

// Ticket grid below cards with columns + rows rendered by ratatui widgets.
#[derive(Debug, Clone )]
pub struct TicketGrid { 
    // todo add fields per D22 ticket grid specification from ADR doc column headers row items etc.  
