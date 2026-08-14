//! [`PlayerPort`] implemented on top of `tauri-plugin-player` (CONTRACTS §7.1).
//!
//! The native player owns playback state (CONTRACTS §1.5); this adapter only
//! turns domain tracks into what the notification / media-session layer needs
//! and keeps a mirror of the queue order, because the plugin exposes no queue
//! getter and `player_queue` (§5) needs one.

use std::sync::{Mutex, MutexGuard};

use tauri::{AppHandle, Runtime};
use tauri_plugin_player::{
    PlaybackState as NativePlaybackState, PlaybackStatus as NativePlaybackStatus, Player,
    QueueItem, RepeatMode as NativeRepeatMode, SetQueueRequest,
};

use crate::application::player_service::PlayerPort;
use crate::domain::{PlaybackState, PlaybackStatus, RepeatMode, Track};
use crate::error::CoreResult;
use crate::infrastructure::android::artwork_file_uri;

pub struct AndroidPlayerAdapter<R: Runtime> {
    app: AppHandle<R>,
    queue: Mutex<Vec<i64>>,
}

impl<R: Runtime> AndroidPlayerAdapter<R> {
    pub fn new(app: AppHandle<R>) -> Self {
        Self {
            app,
            queue: Mutex::new(Vec::new()),
        }
    }

    #[cfg(target_os = "android")]
    fn plugin(&self) -> CoreResult<&Player<R>> {
        use tauri_plugin_player::PlayerExt;
        Ok(self.app.player())
    }

    /// Without Media3 there is nothing to talk to, so every playback call
    /// fails with `PLAYER: unsupported on this platform`.
    #[cfg(not(target_os = "android"))]
    fn plugin(&self) -> CoreResult<&Player<R>> {
        // The handle stays untouched: there is no plugin to reach for.
        let _ = &self.app;
        crate::infrastructure::android::unsupported()
    }

    /// Authoritative refresh, driven by the native `queueChanged` event.
    pub fn sync_queue(&self, track_ids: Vec<i64>) {
        *self.mirror() = track_ids;
    }

    pub fn queue_snapshot(&self) -> Vec<i64> {
        self.mirror().clone()
    }

    fn mirror(&self) -> MutexGuard<'_, Vec<i64>> {
        self.queue
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn items(&self, tracks: &[Track]) -> Vec<QueueItem> {
        tracks.iter().map(|track| self.item(track)).collect()
    }

    /// Artwork is resolved to a `file://` the media session can open; the
    /// WebView-facing URL is a different thing entirely (see `ArtworkPort`).
    fn item(&self, track: &Track) -> QueueItem {
        QueueItem {
            track_id: track.id,
            uri: track.uri.clone(),
            title: track.title.clone(),
            artist: track.artist_name.clone(),
            album: track.album_title.clone(),
            duration_ms: track.duration_ms,
            artwork_uri: track
                .cover_key
                .as_deref()
                .and_then(|key| artwork_file_uri(&self.app, key)),
        }
    }

    /// Slot right after the currently playing item, or the end of the queue
    /// when the player has no current item.
    fn next_slot(&self, player: &Player<R>, len: usize) -> usize {
        player
            .get_state()
            .ok()
            .and_then(|state| state.queue_index)
            .and_then(|index| usize::try_from(index).ok())
            .map(|index| index.saturating_add(1).min(len))
            .unwrap_or(len)
    }
}

#[async_trait::async_trait]
impl<R: Runtime> PlayerPort for AndroidPlayerAdapter<R> {
    async fn state(&self) -> CoreResult<PlaybackState> {
        let Ok(player) = self.plugin() else {
            // Dev builds on the desktop: nothing is loaded, ever.
            return Ok(PlaybackState::idle());
        };
        Ok(playback_state(player.get_state()?))
    }

    async fn queue_ids(&self) -> CoreResult<Vec<i64>> {
        Ok(self.queue_snapshot())
    }

    async fn set_queue(
        &self,
        tracks: &[Track],
        start_index: i32,
        autoplay: bool,
    ) -> CoreResult<()> {
        let player = self.plugin()?;
        player.set_queue(SetQueueRequest {
            items: self.items(tracks),
            start_index,
            autoplay,
        })?;
        *self.mirror() = ids_of(tracks);
        Ok(())
    }

    async fn add_next(&self, tracks: &[Track]) -> CoreResult<()> {
        let player = self.plugin()?;
        player.add_next(self.items(tracks))?;

        let slot = self.next_slot(player, self.mirror().len());
        let mut queue = self.mirror();
        let tail = queue.split_off(slot);
        queue.extend(ids_of(tracks));
        queue.extend(tail);
        Ok(())
    }

    async fn add_to_queue(&self, tracks: &[Track]) -> CoreResult<()> {
        let player = self.plugin()?;
        player.add_to_queue(self.items(tracks))?;
        self.mirror().extend(ids_of(tracks));
        Ok(())
    }

    async fn play(&self) -> CoreResult<()> {
        self.plugin()?.play()?;
        Ok(())
    }

    async fn pause(&self) -> CoreResult<()> {
        self.plugin()?.pause()?;
        Ok(())
    }

    async fn toggle(&self) -> CoreResult<()> {
        self.plugin()?.toggle()?;
        Ok(())
    }

    async fn stop(&self) -> CoreResult<()> {
        self.plugin()?.stop()?;
        Ok(())
    }

    async fn next(&self) -> CoreResult<()> {
        self.plugin()?.next()?;
        Ok(())
    }

    async fn previous(&self) -> CoreResult<()> {
        self.plugin()?.previous()?;
        Ok(())
    }

    async fn seek(&self, position_ms: i64) -> CoreResult<()> {
        self.plugin()?.seek(position_ms)?;
        Ok(())
    }

    async fn skip_to(&self, index: i32) -> CoreResult<()> {
        self.plugin()?.skip_to(index)?;
        Ok(())
    }

    async fn set_shuffle(&self, enabled: bool) -> CoreResult<()> {
        self.plugin()?.set_shuffle(enabled)?;
        Ok(())
    }

    async fn set_repeat(&self, mode: RepeatMode) -> CoreResult<()> {
        self.plugin()?.set_repeat(native_repeat_mode(mode))?;
        Ok(())
    }

    async fn set_volume(&self, volume: f32) -> CoreResult<()> {
        self.plugin()?.set_volume(volume)?;
        Ok(())
    }

    async fn set_speed(&self, speed: f32) -> CoreResult<()> {
        self.plugin()?.set_speed(speed)?;
        Ok(())
    }

    async fn remove_queue_item(&self, index: i32) -> CoreResult<()> {
        self.plugin()?.remove_queue_item(index)?;

        let mut queue = self.mirror();
        if let Some(position) = slot(index, queue.len()) {
            queue.remove(position);
        }
        Ok(())
    }

    async fn move_queue_item(&self, from: i32, to: i32) -> CoreResult<()> {
        self.plugin()?.move_queue_item(from, to)?;

        let mut queue = self.mirror();
        let len = queue.len();
        if let (Some(source), Some(target)) = (slot(from, len), slot(to, len)) {
            let moved = queue.remove(source);
            queue.insert(target, moved);
        }
        Ok(())
    }

    async fn clear_queue(&self) -> CoreResult<()> {
        self.plugin()?.clear_queue()?;
        self.mirror().clear();
        Ok(())
    }
}

fn ids_of(tracks: &[Track]) -> Vec<i64> {
    tracks.iter().map(|track| track.id).collect()
}

/// Native state is copied field by field so a change on either side of the
/// boundary shows up as a compile error rather than as silent drift.
pub fn playback_state(state: NativePlaybackState) -> PlaybackState {
    PlaybackState {
        status: playback_status(state.status),
        track_id: state.track_id,
        position_ms: state.position_ms,
        duration_ms: state.duration_ms,
        queue_index: state.queue_index,
        queue_length: state.queue_length,
        shuffle: state.shuffle,
        repeat: repeat_mode(state.repeat),
        volume: state.volume,
        speed: state.speed,
    }
}

pub fn playback_status(status: NativePlaybackStatus) -> PlaybackStatus {
    match status {
        NativePlaybackStatus::Idle => PlaybackStatus::Idle,
        NativePlaybackStatus::Buffering => PlaybackStatus::Buffering,
        NativePlaybackStatus::Playing => PlaybackStatus::Playing,
        NativePlaybackStatus::Paused => PlaybackStatus::Paused,
        NativePlaybackStatus::Ended => PlaybackStatus::Ended,
    }
}

pub fn repeat_mode(mode: NativeRepeatMode) -> RepeatMode {
    match mode {
        NativeRepeatMode::Off => RepeatMode::Off,
        NativeRepeatMode::All => RepeatMode::All,
        NativeRepeatMode::One => RepeatMode::One,
    }
}

pub fn native_repeat_mode(mode: RepeatMode) -> NativeRepeatMode {
    match mode {
        RepeatMode::Off => NativeRepeatMode::Off,
        RepeatMode::All => NativeRepeatMode::All,
        RepeatMode::One => NativeRepeatMode::One,
    }
}

/// Index coming from the frontend, resolved against the mirror.
fn slot(index: i32, len: usize) -> Option<usize> {
    let position = usize::try_from(index).ok()?;
    (position < len).then_some(position)
}

#[cfg(test)]
mod tests {
    use super::{ids_of, native_repeat_mode, playback_state, playback_status, repeat_mode, slot};
    use crate::application::testing::track;
    use crate::domain::{PlaybackStatus, RepeatMode};
    use tauri_plugin_player::{
        PlaybackState as NativePlaybackState, PlaybackStatus as NativePlaybackStatus,
        RepeatMode as NativeRepeatMode,
    };

    #[test]
    fn slots_reject_out_of_range_indices() {
        assert_eq!(slot(0, 3), Some(0));
        assert_eq!(slot(2, 3), Some(2));
        assert_eq!(slot(3, 3), None);
        assert_eq!(slot(-1, 3), None);
        assert_eq!(slot(0, 0), None);
    }

    #[test]
    fn queue_order_is_taken_from_the_tracks() {
        let tracks = vec![track(3), track(1), track(3)];
        assert_eq!(ids_of(&tracks), vec![3, 1, 3], "duplicates are kept");
        assert!(ids_of(&[]).is_empty());
    }

    #[test]
    fn state_is_translated_field_by_field() {
        let state = playback_state(NativePlaybackState {
            status: NativePlaybackStatus::Playing,
            track_id: Some(9),
            position_ms: 1_500,
            duration_ms: 200_000,
            queue_index: Some(1),
            queue_length: 4,
            shuffle: true,
            repeat: NativeRepeatMode::All,
            volume: 0.5,
            speed: 1.25,
        });

        assert_eq!(state.status, PlaybackStatus::Playing);
        assert_eq!(state.track_id, Some(9));
        assert_eq!(state.position_ms, 1_500);
        assert_eq!(state.duration_ms, 200_000);
        assert_eq!(state.queue_index, Some(1));
        assert_eq!(state.queue_length, 4);
        assert!(state.shuffle);
        assert_eq!(state.repeat, RepeatMode::All);
        assert!((state.volume - 0.5).abs() < f32::EPSILON);
        assert!((state.speed - 1.25).abs() < f32::EPSILON);
    }

    #[test]
    fn enums_map_both_ways() {
        assert_eq!(
            playback_status(NativePlaybackStatus::Ended),
            PlaybackStatus::Ended
        );
        assert_eq!(
            playback_status(NativePlaybackStatus::Buffering),
            PlaybackStatus::Buffering
        );
        for (domain, native) in [
            (RepeatMode::Off, NativeRepeatMode::Off),
            (RepeatMode::All, NativeRepeatMode::All),
            (RepeatMode::One, NativeRepeatMode::One),
        ] {
            assert_eq!(repeat_mode(native), domain);
            assert_eq!(native_repeat_mode(domain), native);
        }
    }
}
