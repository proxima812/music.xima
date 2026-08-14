use serde::{Deserialize, Serialize};

use crate::domain::Track;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HiddenTrack {
    pub track: Track,
    pub hidden_at: i64,
}

impl Eq for HiddenTrack {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PendingDeletion {
    pub track_id: i64,
    pub uri: String,
    pub requested_at: i64,
    pub file_deleted: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DeleteTrackResult {
    Deleted,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileDeleteOutcome {
    Deleted,
    Cancelled,
}
