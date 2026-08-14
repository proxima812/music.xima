//! Desktop stub (CONTRACTS §7.3).
//!
//! There is no Media3 outside Android, so playback answers `Unsupported` and
//! the library side answers "nothing found". This exists so `cargo check`,
//! `cargo test` and the dev build of the core crate work on macOS/Linux.

use std::marker::PhantomData;

use serde::de::DeserializeOwned;
use tauri::{plugin::PluginApi, AppHandle, Runtime};

use crate::error::{Error, Result};
use crate::models::{
    PlaybackState, PlayerEvent, QueueItem, RepeatMode, ScanBatch, SetQueueRequest,
};

pub fn init<R: Runtime, C: DeserializeOwned>(
    _app: &AppHandle<R>,
    _api: PluginApi<R, C>,
) -> Result<Player<R>> {
    Ok(Player(PhantomData))
}

/// `fn() -> R` keeps the struct `Send + Sync` without constraining `R`.
pub struct Player<R: Runtime>(PhantomData<fn() -> R>);

impl<R: Runtime> Player<R> {
    pub fn get_state(&self) -> Result<PlaybackState> {
        Ok(PlaybackState::idle())
    }

    pub fn set_queue(&self, _req: SetQueueRequest) -> Result<()> {
        Err(Error::Unsupported)
    }

    pub fn play(&self) -> Result<()> {
        Err(Error::Unsupported)
    }

    pub fn pause(&self) -> Result<()> {
        Err(Error::Unsupported)
    }

    pub fn toggle(&self) -> Result<()> {
        Err(Error::Unsupported)
    }

    pub fn stop(&self) -> Result<()> {
        Err(Error::Unsupported)
    }

    pub fn next(&self) -> Result<()> {
        Err(Error::Unsupported)
    }

    pub fn previous(&self) -> Result<()> {
        Err(Error::Unsupported)
    }

    pub fn seek(&self, _position_ms: i64) -> Result<()> {
        Err(Error::Unsupported)
    }

    pub fn skip_to(&self, _index: i32) -> Result<()> {
        Err(Error::Unsupported)
    }

    pub fn set_shuffle(&self, _enabled: bool) -> Result<()> {
        Err(Error::Unsupported)
    }

    pub fn set_repeat(&self, _mode: RepeatMode) -> Result<()> {
        Err(Error::Unsupported)
    }

    pub fn set_volume(&self, _volume: f32) -> Result<()> {
        Err(Error::Unsupported)
    }

    pub fn set_speed(&self, _speed: f32) -> Result<()> {
        Err(Error::Unsupported)
    }

    pub fn add_next(&self, _items: Vec<QueueItem>) -> Result<()> {
        Err(Error::Unsupported)
    }

    pub fn add_to_queue(&self, _items: Vec<QueueItem>) -> Result<()> {
        Err(Error::Unsupported)
    }

    pub fn remove_queue_item(&self, _index: i32) -> Result<()> {
        Err(Error::Unsupported)
    }

    pub fn move_queue_item(&self, _from: i32, _to: i32) -> Result<()> {
        Err(Error::Unsupported)
    }

    pub fn clear_queue(&self) -> Result<()> {
        Err(Error::Unsupported)
    }

    pub fn scan_media_store(&self, _since: Option<i64>) -> Result<ScanBatch> {
        Ok(ScanBatch::empty())
    }

    pub fn scan_tree(&self, _tree_uri: String, _since: Option<i64>) -> Result<ScanBatch> {
        Ok(ScanBatch::empty())
    }

    pub fn pick_folder(&self) -> Result<Option<String>> {
        Ok(None)
    }

    pub fn persisted_roots(&self) -> Result<Vec<String>> {
        Ok(Vec::new())
    }

    pub fn release_root(&self, _tree_uri: String) -> Result<()> {
        Ok(())
    }

    pub fn extract_artwork(&self, _uri: String) -> Result<Option<String>> {
        Ok(None)
    }

    /// Nothing ever fires here: the desktop stub has no native side to listen
    /// to. The handler is dropped immediately.
    pub fn on_event<F>(&self, _handler: F) -> Result<()>
    where
        F: Fn(PlayerEvent) + Send + Sync + 'static,
    {
        Ok(())
    }
}
