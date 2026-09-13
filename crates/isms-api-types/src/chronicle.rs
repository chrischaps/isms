//! Chronicle and communication wire types (TDD 10.3 Society and Comms rows, S1.5).

use crate::Clock;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct HeadlineView {
    /// The event the headline was written from (`/explain/{seq}`).
    pub seq: i64,
    pub ordinal: i32,
    pub cycle: u32,
    pub tick: u32,
    pub text: String,
}

/// One cycle's edition of the Chronicle.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ChronicleView {
    pub clock: Clock,
    /// The cycle shown (1-based); the current one by default.
    pub cycle: u32,
    pub headlines: Vec<HeadlineView>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct MessageView {
    pub id: i64,
    pub channel: String,
    pub sender: u32,
    pub handle: String,
    pub body: String,
    pub tick: u32,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct MessagesView {
    pub clock: Clock,
    /// `square`, `org:<id>`, or `dm:<a>:<b>`.
    pub channel: String,
    pub messages: Vec<MessageView>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct PostMessage {
    pub body: String,
}
