//! Playback orchestration (CONTRACTS §5, `player_*`).
//!
//! The native player owns playback state (CONTRACTS §1.5), so this service
//! never derives or caches it. What it does own is the step the native side
//! cannot take: turning library ids into the tracks the player needs, and
//! keeping the resulting queue order around so `player_queue` can be answered.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

use crate::application::{validated_index, validated_track_ids};
use crate::domain::{PlaybackState, RepeatMode, Track};
use crate::error::{CoreError, CoreResult};
use crate::infrastructure::repositories::TrackRepository;

/// Bounds the native player accepts. A slider that overshoots by a rounding
/// error is clamped rather than rejected; only garbage (`NaN`, infinity) is an
/// error.
pub const MIN_VOLUME: f32 = 0.0;
pub const MAX_VOLUME: f32 = 1.0;
pub const MIN_SPEED: f32 = 0.25;
pub const MAX_SPEED: f32 = 4.0;

/// The native player as the application layer sees it: CONTRACTS §7.1 with the
/// plugin types replaced by domain types, plus `queue_ids`, because the plugin
/// exposes no queue getter and `player_queue` (§5) needs one.
///
/// Implemented in `infrastructure::android`; the mapping from [`Track`] to the
/// plugin's `QueueItem` lives there, because resolving `cover_key` to an
/// artwork URI needs the app cache directory.
#[async_trait::async_trait]
pub trait PlayerPort: Send + Sync {
    async fn state(&self) -> CoreResult<PlaybackState>;
    /// Track ids currently loaded in the player, in queue order.
    async fn queue_ids(&self) -> CoreResult<Vec<i64>>;
    async fn set_queue(&self, tracks: &[Track], start_index: i32, autoplay: bool)
        -> CoreResult<()>;
    async fn add_next(&self, tracks: &[Track]) -> CoreResult<()>;
    async fn add_to_queue(&self, tracks: &[Track]) -> CoreResult<()>;
    async fn play(&self) -> CoreResult<()>;
    async fn pause(&self) -> CoreResult<()>;
    async fn toggle(&self) -> CoreResult<()>;
    async fn stop(&self) -> CoreResult<()>;
    async fn next(&self) -> CoreResult<()>;
    async fn previous(&self) -> CoreResult<()>;
    async fn seek(&self, position_ms: i64) -> CoreResult<()>;
    async fn skip_to(&self, index: i32) -> CoreResult<()>;
    async fn set_shuffle(&self, enabled: bool) -> CoreResult<()>;
    async fn set_repeat(&self, mode: RepeatMode) -> CoreResult<()>;
    async fn set_volume(&self, volume: f32) -> CoreResult<()>;
    async fn set_speed(&self, speed: f32) -> CoreResult<()>;
    async fn remove_queue_item(&self, index: i32) -> CoreResult<()>;
    async fn move_queue_item(&self, from: i32, to: i32) -> CoreResult<()>;
    async fn clear_queue(&self) -> CoreResult<()>;
}

pub struct PlayerService {
    tracks: Arc<dyn TrackRepository>,
    player: Arc<dyn PlayerPort>,
    /// Queue order as this service last left it. Only a mirror: the player is
    /// still the source of truth, see [`PlayerService::queue`].
    queue: Mutex<Vec<i64>>,
}

impl PlayerService {
    pub fn new(tracks: Arc<dyn TrackRepository>, player: Arc<dyn PlayerPort>) -> Self {
        Self {
            tracks,
            player,
            queue: Mutex::new(Vec::new()),
        }
    }

    pub async fn state(&self) -> CoreResult<PlaybackState> {
        self.player.state().await
    }

    /// Replaces the queue. Ids that no longer resolve to a row are dropped, so
    /// a stale selection still plays what is left of it.
    pub async fn set_queue(
        &self,
        track_ids: &[i64],
        start_index: i32,
        autoplay: bool,
    ) -> CoreResult<()> {
        let tracks = self.load(track_ids).await?;
        let start_index = clamp_start_index(start_index, tracks.len())?;
        self.player
            .set_queue(&tracks, start_index, autoplay)
            .await?;
        *self.mirror() = ids_of(&tracks);
        Ok(())
    }

    /// Plays a selection from its first track — "play all" / "play album".
    pub async fn play_tracks(&self, track_ids: &[i64]) -> CoreResult<()> {
        self.set_queue(track_ids, 0, true).await
    }

    /// The queue as full tracks, in playback order. The player's own mirror
    /// wins because the `queueChanged` event keeps it authoritative; the local
    /// copy answers ports that keep no mirror of their own.
    pub async fn queue(&self) -> CoreResult<Vec<Track>> {
        let mut ids = self.player.queue_ids().await?;
        if ids.is_empty() {
            ids = self.queue_snapshot();
        }
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let tracks = self.tracks.get_many(&ids).await?;
        Ok(ordered(&ids, tracks))
    }

    /// Queue order as this service knows it.
    pub fn queue_snapshot(&self) -> Vec<i64> {
        self.mirror().clone()
    }

    /// Authoritative refresh, driven by the native `queueChanged` event
    /// (CONTRACTS §6, `player:queue-changed`).
    pub fn sync_queue(&self, track_ids: Vec<i64>) {
        *self.mirror() = track_ids;
    }

    pub async fn play(&self) -> CoreResult<()> {
        self.player.play().await
    }

    pub async fn pause(&self) -> CoreResult<()> {
        self.player.pause().await
    }

    pub async fn toggle(&self) -> CoreResult<()> {
        self.player.toggle().await
    }

    pub async fn stop(&self) -> CoreResult<()> {
        self.player.stop().await
    }

    pub async fn next(&self) -> CoreResult<()> {
        self.player.next().await
    }

    pub async fn previous(&self) -> CoreResult<()> {
        self.player.previous().await
    }

    pub async fn seek(&self, position_ms: i64) -> CoreResult<()> {
        validated_index("positionMs", position_ms)?;
        self.player.seek(position_ms).await
    }

    pub async fn skip_to(&self, index: i32) -> CoreResult<()> {
        validated_index("index", i64::from(index))?;
        self.player.skip_to(index).await
    }

    pub async fn set_shuffle(&self, enabled: bool) -> CoreResult<()> {
        self.player.set_shuffle(enabled).await
    }

    pub async fn set_repeat(&self, mode: RepeatMode) -> CoreResult<()> {
        self.player.set_repeat(mode).await
    }

    pub async fn set_volume(&self, volume: f32) -> CoreResult<()> {
        let volume = clamped("volume", volume, MIN_VOLUME, MAX_VOLUME)?;
        self.player.set_volume(volume).await
    }

    pub async fn set_speed(&self, speed: f32) -> CoreResult<()> {
        let speed = clamped("speed", speed, MIN_SPEED, MAX_SPEED)?;
        self.player.set_speed(speed).await
    }

    /// Inserts right after the current item.
    pub async fn add_next(&self, track_ids: &[i64]) -> CoreResult<()> {
        let tracks = self.load(track_ids).await?;
        let current = self.current_index().await;
        self.player.add_next(&tracks).await?;

        let mut queue = self.mirror();
        let end = queue.len();
        let slot = current.map_or(end, |index| index.saturating_add(1).min(end));
        let tail = queue.split_off(slot);
        queue.extend(ids_of(&tracks));
        queue.extend(tail);
        Ok(())
    }

    pub async fn add_to_queue(&self, track_ids: &[i64]) -> CoreResult<()> {
        let tracks = self.load(track_ids).await?;
        self.player.add_to_queue(&tracks).await?;
        self.mirror().extend(ids_of(&tracks));
        Ok(())
    }

    pub async fn remove_queue_item(&self, index: i32) -> CoreResult<()> {
        validated_index("index", i64::from(index))?;
        self.player.remove_queue_item(index).await?;

        let mut queue = self.mirror();
        if let Some(position) = slot(index, queue.len()) {
            queue.remove(position);
        }
        Ok(())
    }

    pub async fn move_queue_item(&self, from: i32, to: i32) -> CoreResult<()> {
        validated_index("from", i64::from(from))?;
        validated_index("to", i64::from(to))?;
        if from == to {
            return Ok(());
        }
        self.player.move_queue_item(from, to).await?;

        let mut queue = self.mirror();
        let len = queue.len();
        if let (Some(source), Some(target)) = (slot(from, len), slot(to, len)) {
            let moved = queue.remove(source);
            queue.insert(target, moved);
        }
        Ok(())
    }

    pub async fn clear_queue(&self) -> CoreResult<()> {
        self.player.clear_queue().await?;
        self.mirror().clear();
        Ok(())
    }

    /// Ids -> tracks, in the order they were asked for. Missing rows are
    /// skipped; a request that resolves to nothing at all is an error, because
    /// handing the player an empty queue would silently stop playback.
    async fn load(&self, track_ids: &[i64]) -> CoreResult<Vec<Track>> {
        if track_ids.is_empty() {
            return Err(CoreError::invalid_input("trackIds must not be empty"));
        }
        validated_track_ids(track_ids)?;

        let tracks = ordered(track_ids, self.tracks.get_many(track_ids).await?);
        if tracks.is_empty() {
            return Err(CoreError::NotFound(
                "none of the requested tracks".to_owned(),
            ));
        }
        Ok(tracks)
    }

    /// Best effort: a player that cannot report its position also cannot tell
    /// us where "next" is, and the queue mirror falls back to appending.
    async fn current_index(&self) -> Option<usize> {
        self.player
            .state()
            .await
            .ok()
            .and_then(|state| state.queue_index)
            .and_then(|index| usize::try_from(index).ok())
    }

    fn mirror(&self) -> MutexGuard<'_, Vec<i64>> {
        self.queue
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

fn ids_of(tracks: &[Track]) -> Vec<i64> {
    tracks.iter().map(|track| track.id).collect()
}

/// Keeps the requested order and keeps duplicates: the same track may sit in
/// the queue more than once.
fn ordered(ids: &[i64], tracks: Vec<Track>) -> Vec<Track> {
    let by_id: HashMap<i64, Track> = tracks.into_iter().map(|track| (track.id, track)).collect();
    ids.iter().filter_map(|id| by_id.get(id).cloned()).collect()
}

/// A start index past the end means "start at the last track", which is what a
/// stale UI selection amounts to.
fn clamp_start_index(start_index: i32, len: usize) -> CoreResult<i32> {
    if start_index < 0 {
        return Err(CoreError::invalid_input("startIndex must not be negative"));
    }
    let last = i32::try_from(len.saturating_sub(1)).unwrap_or(i32::MAX);
    Ok(start_index.min(last))
}

fn clamped(name: &str, value: f32, min: f32, max: f32) -> CoreResult<f32> {
    if !value.is_finite() {
        return Err(CoreError::invalid_input(format!(
            "{name} must be a finite number"
        )));
    }
    Ok(value.clamp(min, max))
}

/// Index coming from the frontend, resolved against the mirror.
fn slot(index: i32, len: usize) -> Option<usize> {
    let position = usize::try_from(index).ok()?;
    (position < len).then_some(position)
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::{PlayerPort, PlayerService, MAX_SPEED, MAX_VOLUME, MIN_SPEED};
    use crate::application::testing::track;
    use crate::domain::{
        LibraryStats, Page, PlaybackState, RepeatMode, ScannedTrack, Track, TrackQuery,
    };
    use crate::error::{CoreError, CoreResult};
    use crate::infrastructure::repositories::TrackRepository;

    /// What the port was asked to do, in order.
    #[derive(Debug, Clone, PartialEq)]
    enum Call {
        SetQueue(Vec<i64>, i32, bool),
        AddNext(Vec<i64>),
        AddToQueue(Vec<i64>),
        Seek(i64),
        SkipTo(i32),
        SetVolume(f32),
        SetSpeed(f32),
        SetRepeat(RepeatMode),
        Remove(i32),
        Move(i32, i32),
        Clear,
    }

    #[derive(Default)]
    struct FakePlayer {
        calls: Mutex<Vec<Call>>,
        /// Mirrors what the native side would report; `None` = no current item.
        queue_index: Mutex<Option<i32>>,
        /// Empty means "this port keeps no mirror", which is the fallback case.
        queue_ids: Mutex<Vec<i64>>,
    }

    impl FakePlayer {
        fn calls(&self) -> Vec<Call> {
            self.calls.lock().expect("lock").clone()
        }

        fn record(&self, call: Call) {
            self.calls.lock().expect("lock").push(call);
        }
    }

    #[async_trait::async_trait]
    impl PlayerPort for FakePlayer {
        async fn state(&self) -> CoreResult<PlaybackState> {
            Ok(PlaybackState {
                queue_index: *self.queue_index.lock().expect("lock"),
                ..PlaybackState::idle()
            })
        }

        async fn queue_ids(&self) -> CoreResult<Vec<i64>> {
            Ok(self.queue_ids.lock().expect("lock").clone())
        }

        async fn set_queue(
            &self,
            tracks: &[Track],
            start_index: i32,
            autoplay: bool,
        ) -> CoreResult<()> {
            self.record(Call::SetQueue(
                tracks.iter().map(|track| track.id).collect(),
                start_index,
                autoplay,
            ));
            Ok(())
        }

        async fn add_next(&self, tracks: &[Track]) -> CoreResult<()> {
            self.record(Call::AddNext(tracks.iter().map(|t| t.id).collect()));
            Ok(())
        }

        async fn add_to_queue(&self, tracks: &[Track]) -> CoreResult<()> {
            self.record(Call::AddToQueue(tracks.iter().map(|t| t.id).collect()));
            Ok(())
        }

        async fn play(&self) -> CoreResult<()> {
            Ok(())
        }

        async fn pause(&self) -> CoreResult<()> {
            Ok(())
        }

        async fn toggle(&self) -> CoreResult<()> {
            Ok(())
        }

        async fn stop(&self) -> CoreResult<()> {
            Ok(())
        }

        async fn next(&self) -> CoreResult<()> {
            Ok(())
        }

        async fn previous(&self) -> CoreResult<()> {
            Ok(())
        }

        async fn seek(&self, position_ms: i64) -> CoreResult<()> {
            self.record(Call::Seek(position_ms));
            Ok(())
        }

        async fn skip_to(&self, index: i32) -> CoreResult<()> {
            self.record(Call::SkipTo(index));
            Ok(())
        }

        async fn set_shuffle(&self, _enabled: bool) -> CoreResult<()> {
            Ok(())
        }

        async fn set_repeat(&self, mode: RepeatMode) -> CoreResult<()> {
            self.record(Call::SetRepeat(mode));
            Ok(())
        }

        async fn set_volume(&self, volume: f32) -> CoreResult<()> {
            self.record(Call::SetVolume(volume));
            Ok(())
        }

        async fn set_speed(&self, speed: f32) -> CoreResult<()> {
            self.record(Call::SetSpeed(speed));
            Ok(())
        }

        async fn remove_queue_item(&self, index: i32) -> CoreResult<()> {
            self.record(Call::Remove(index));
            Ok(())
        }

        async fn move_queue_item(&self, from: i32, to: i32) -> CoreResult<()> {
            self.record(Call::Move(from, to));
            Ok(())
        }

        async fn clear_queue(&self) -> CoreResult<()> {
            self.record(Call::Clear);
            Ok(())
        }
    }

    /// Knows tracks 1..=5; everything else is missing.
    #[derive(Default)]
    struct FakeTracks {
        requested: Mutex<Vec<Vec<i64>>>,
    }

    #[async_trait::async_trait]
    impl TrackRepository for FakeTracks {
        async fn get(&self, id: i64) -> CoreResult<Track> {
            Ok(track(id))
        }

        async fn get_many(&self, ids: &[i64]) -> CoreResult<Vec<Track>> {
            self.requested.lock().expect("lock").push(ids.to_vec());
            let mut found: Vec<Track> = ids
                .iter()
                .copied()
                .filter(|id| (1..=5).contains(id))
                .map(track)
                .collect();
            // The repository makes no ordering promise and returns each row
            // once; ordering and duplicates are the service's job.
            found.sort_by_key(|row| -row.id);
            found.dedup_by_key(|row| row.id);
            Ok(found)
        }

        async fn query(&self, q: &TrackQuery) -> CoreResult<Page<Track>> {
            Ok(Page::empty(q.offset, q.limit))
        }

        async fn recently_added(&self, _limit: i64) -> CoreResult<Vec<Track>> {
            Ok(Vec::new())
        }

        async fn upsert_many(&self, _tracks: &[ScannedTrack]) -> CoreResult<u64> {
            Ok(0)
        }

        async fn delete_missing(&self, _keep_uris: &[String]) -> CoreResult<u64> {
            Ok(0)
        }

        async fn stats(&self) -> CoreResult<LibraryStats> {
            Ok(LibraryStats::default())
        }
    }

    fn service() -> (PlayerService, Arc<FakeTracks>, Arc<FakePlayer>) {
        let tracks = Arc::new(FakeTracks::default());
        let player = Arc::new(FakePlayer::default());
        (
            PlayerService::new(tracks.clone(), player.clone()),
            tracks,
            player,
        )
    }

    #[tokio::test]
    async fn set_queue_hands_the_player_tracks_in_the_requested_order() {
        let (service, tracks, player) = service();

        service
            .set_queue(&[3, 1, 2], 1, true)
            .await
            .expect("queued");

        assert_eq!(player.calls(), vec![Call::SetQueue(vec![3, 1, 2], 1, true)]);
        assert_eq!(
            tracks.requested.lock().expect("lock").as_slice(),
            &[vec![3, 1, 2]]
        );
        assert_eq!(service.queue_snapshot(), vec![3, 1, 2]);
    }

    #[tokio::test]
    async fn queue_items_carry_everything_the_notification_needs() {
        let (service, _, player) = service();

        // Playing a stale selection: id 99 no longer exists and is dropped.
        service.play_tracks(&[2, 99, 2]).await.expect("playing");

        let calls = player.calls();
        assert_eq!(calls, vec![Call::SetQueue(vec![2, 2], 0, true)]);
        assert_eq!(service.queue_snapshot(), vec![2, 2], "duplicates survive");

        // This port reports no queue of its own, so the mirror answers.
        let queue = service.queue().await.expect("queue");
        assert_eq!(queue.len(), 2);
        let first = queue.first().expect("first item");
        assert_eq!(first.id, 2);
        assert_eq!(first.uri, "content://media/external/audio/media/2");
        assert_eq!(first.title, "Track 2");
        assert_eq!(first.artist_name.as_deref(), Some("Artist"));
        assert_eq!(first.album_title.as_deref(), Some("Album"));
        assert_eq!(first.duration_ms, 200_002);
        assert_eq!(first.cover_key.as_deref(), Some("cover-2"));
    }

    #[tokio::test]
    async fn queue_prefers_what_the_player_reports() {
        let (service, _, player) = service();
        service.set_queue(&[1, 2], 0, false).await.expect("queued");
        *player.queue_ids.lock().expect("lock") = vec![2, 1];

        let queue = service.queue().await.expect("queue");

        assert_eq!(
            queue.iter().map(|track| track.id).collect::<Vec<i64>>(),
            vec![2, 1]
        );
    }

    #[tokio::test]
    async fn set_queue_rejects_empty_and_unknown_selections() {
        let (service, _, player) = service();

        assert_eq!(
            service
                .set_queue(&[], 0, true)
                .await
                .expect_err("empty selection")
                .code(),
            "INVALID_INPUT"
        );
        assert_eq!(
            service
                .set_queue(&[1, -2], 0, true)
                .await
                .expect_err("negative id")
                .code(),
            "INVALID_INPUT"
        );
        assert_eq!(
            service
                .set_queue(&[7, 8], 0, true)
                .await
                .expect_err("nothing resolves")
                .code(),
            "NOT_FOUND"
        );
        assert_eq!(
            service
                .set_queue(&[1], -1, true)
                .await
                .expect_err("negative start index")
                .code(),
            "INVALID_INPUT"
        );
        assert!(player.calls().is_empty());
        assert!(service.queue_snapshot().is_empty());
    }

    #[tokio::test]
    async fn start_index_past_the_end_lands_on_the_last_track() {
        let (service, _, player) = service();

        service.set_queue(&[1, 2], 9, false).await.expect("queued");

        assert_eq!(player.calls(), vec![Call::SetQueue(vec![1, 2], 1, false)]);
    }

    #[tokio::test]
    async fn add_next_inserts_after_the_current_item() {
        let (service, _, player) = service();
        service
            .set_queue(&[1, 2, 3], 0, true)
            .await
            .expect("queued");
        *player.queue_index.lock().expect("lock") = Some(1);

        service.add_next(&[4, 5]).await.expect("added");

        assert_eq!(service.queue_snapshot(), vec![1, 2, 4, 5, 3]);
        assert_eq!(player.calls().last(), Some(&Call::AddNext(vec![4, 5])));
    }

    #[tokio::test]
    async fn add_next_without_a_current_item_appends() {
        let (service, _, player) = service();
        service.set_queue(&[1, 2], 0, false).await.expect("queued");
        *player.queue_index.lock().expect("lock") = None;

        service.add_next(&[3]).await.expect("added");
        service.add_to_queue(&[4]).await.expect("added");

        assert_eq!(service.queue_snapshot(), vec![1, 2, 3, 4]);
        assert_eq!(player.calls().last(), Some(&Call::AddToQueue(vec![4])));
    }

    #[tokio::test]
    async fn queue_edits_keep_the_mirror_in_step() {
        let (service, _, _) = service();
        service
            .set_queue(&[1, 2, 3, 4], 0, false)
            .await
            .expect("queued");

        service.move_queue_item(0, 2).await.expect("moved");
        assert_eq!(service.queue_snapshot(), vec![2, 3, 1, 4]);

        service.remove_queue_item(1).await.expect("removed");
        assert_eq!(service.queue_snapshot(), vec![2, 1, 4]);

        // Out of range on the native side too: nothing to do, no error.
        service.remove_queue_item(9).await.expect("out of range");
        assert_eq!(service.queue_snapshot(), vec![2, 1, 4]);

        service.clear_queue().await.expect("cleared");
        assert!(service.queue_snapshot().is_empty());
    }

    #[tokio::test]
    async fn queue_edits_reject_negative_indexes() {
        let (service, _, player) = service();

        assert_eq!(
            service
                .remove_queue_item(-1)
                .await
                .expect_err("negative index")
                .code(),
            "INVALID_INPUT"
        );
        assert_eq!(
            service
                .move_queue_item(0, -3)
                .await
                .expect_err("negative target")
                .code(),
            "INVALID_INPUT"
        );
        assert_eq!(
            service.seek(-1).await.expect_err("negative seek").code(),
            "INVALID_INPUT"
        );
        assert_eq!(
            service
                .skip_to(-2)
                .await
                .expect_err("negative index")
                .code(),
            "INVALID_INPUT"
        );

        service.move_queue_item(2, 2).await.expect("no-op move");
        assert!(player.calls().is_empty());
    }

    #[tokio::test]
    async fn volume_and_speed_are_clamped_and_reject_garbage() {
        let (service, _, player) = service();

        service.set_volume(1.5).await.expect("clamped");
        service.set_volume(-0.2).await.expect("clamped");
        service.set_speed(10.0).await.expect("clamped");
        service.set_speed(0.01).await.expect("clamped");
        service.seek(1_000).await.expect("sought");
        service.skip_to(2).await.expect("skipped");
        service
            .set_repeat(RepeatMode::All)
            .await
            .expect("repeat set");

        assert_eq!(
            service
                .set_volume(f32::NAN)
                .await
                .expect_err("not a number")
                .code(),
            "INVALID_INPUT"
        );
        assert_eq!(
            service
                .set_speed(f32::INFINITY)
                .await
                .expect_err("infinite")
                .code(),
            "INVALID_INPUT"
        );

        assert_eq!(
            player.calls(),
            vec![
                Call::SetVolume(MAX_VOLUME),
                Call::SetVolume(0.0),
                Call::SetSpeed(MAX_SPEED),
                Call::SetSpeed(MIN_SPEED),
                Call::Seek(1_000),
                Call::SkipTo(2),
                Call::SetRepeat(RepeatMode::All),
            ]
        );
    }

    #[tokio::test]
    async fn a_failing_player_leaves_the_mirror_alone() {
        struct BrokenPlayer;

        #[async_trait::async_trait]
        impl PlayerPort for BrokenPlayer {
            async fn state(&self) -> CoreResult<PlaybackState> {
                Ok(PlaybackState::idle())
            }

            async fn queue_ids(&self) -> CoreResult<Vec<i64>> {
                Ok(Vec::new())
            }

            async fn set_queue(
                &self,
                _tracks: &[Track],
                _start_index: i32,
                _autoplay: bool,
            ) -> CoreResult<()> {
                Err(CoreError::Player("unsupported on this platform".to_owned()))
            }

            async fn add_next(&self, _tracks: &[Track]) -> CoreResult<()> {
                Err(CoreError::Player("unsupported".to_owned()))
            }

            async fn add_to_queue(&self, _tracks: &[Track]) -> CoreResult<()> {
                Err(CoreError::Player("unsupported".to_owned()))
            }

            async fn play(&self) -> CoreResult<()> {
                Err(CoreError::Player("unsupported".to_owned()))
            }

            async fn pause(&self) -> CoreResult<()> {
                Ok(())
            }

            async fn toggle(&self) -> CoreResult<()> {
                Ok(())
            }

            async fn stop(&self) -> CoreResult<()> {
                Ok(())
            }

            async fn next(&self) -> CoreResult<()> {
                Ok(())
            }

            async fn previous(&self) -> CoreResult<()> {
                Ok(())
            }

            async fn seek(&self, _position_ms: i64) -> CoreResult<()> {
                Ok(())
            }

            async fn skip_to(&self, _index: i32) -> CoreResult<()> {
                Ok(())
            }

            async fn set_shuffle(&self, _enabled: bool) -> CoreResult<()> {
                Ok(())
            }

            async fn set_repeat(&self, _mode: RepeatMode) -> CoreResult<()> {
                Ok(())
            }

            async fn set_volume(&self, _volume: f32) -> CoreResult<()> {
                Ok(())
            }

            async fn set_speed(&self, _speed: f32) -> CoreResult<()> {
                Ok(())
            }

            async fn remove_queue_item(&self, _index: i32) -> CoreResult<()> {
                Ok(())
            }

            async fn move_queue_item(&self, _from: i32, _to: i32) -> CoreResult<()> {
                Ok(())
            }

            async fn clear_queue(&self) -> CoreResult<()> {
                Ok(())
            }
        }

        let service = PlayerService::new(Arc::new(FakeTracks::default()), Arc::new(BrokenPlayer));

        let error = service
            .set_queue(&[1, 2], 0, true)
            .await
            .expect_err("player refused");

        assert_eq!(error.code(), "PLAYER");
        assert!(service.queue_snapshot().is_empty());
        assert!(service.queue().await.expect("empty queue").is_empty());
    }

    #[tokio::test]
    async fn sync_queue_replaces_the_mirror() {
        let (service, _, _) = service();
        service.set_queue(&[1, 2], 0, false).await.expect("queued");

        service.sync_queue(vec![5, 4, 3]);

        assert_eq!(service.queue_snapshot(), vec![5, 4, 3]);
    }
}
