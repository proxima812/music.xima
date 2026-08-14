//! Tauri command layer (CONTRACTS §5).
//!
//! Every function is a shell: it carries the exact command name from the
//! contract, unpacks the arguments Tauri decoded, calls one application service
//! and returns the DTO. No SQL, no branching on domain rules, no state of its
//! own — everything worth testing lives one layer down.
//!
//! Errors leave as [`crate::error::CoreError`], which serializes into
//! `{ code, message }` (CONTRACTS §2).

pub mod favorites;
pub mod history;
pub mod library;
pub mod player;
pub mod playlist;
pub mod search;
pub mod statistics;
pub mod track_removal;
