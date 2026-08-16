//! Queue-safe hiding and permanent track deletion orchestration.

use std::sync::Arc;

use crate::application::{Clock, PlayerService, SystemClock};
use crate::domain::{DeleteTrackResult, FileDeleteOutcome, HiddenTrack, PendingDeletion};
use crate::error::{CoreError, CoreResult};
use crate::infrastructure::repositories::TrackRepository;

#[async_trait::async_trait]
pub trait TrackFilePort: Send + Sync {
    async fn delete(&self, uri: &str) -> CoreResult<FileDeleteOutcome>;
    async fn exists(&self, uri: &str) -> CoreResult<bool>;
}

pub type RecoveryFailure = (i64, CoreError);

pub struct RecoveryReport {
    pub finalized_count: usize,
    pub failures: Vec<RecoveryFailure>,
}

pub struct TrackRemovalService {
    tracks: Arc<dyn TrackRepository>,
    files: Arc<dyn TrackFilePort>,
    player: Arc<PlayerService>,
    clock: Arc<dyn Clock>,
}

impl TrackRemovalService {
    pub fn new(
        tracks: Arc<dyn TrackRepository>,
        files: Arc<dyn TrackFilePort>,
        player: Arc<PlayerService>,
    ) -> Self {
        Self::with_clock(tracks, files, player, Arc::new(SystemClock))
    }

    pub fn with_clock(
        tracks: Arc<dyn TrackRepository>,
        files: Arc<dyn TrackFilePort>,
        player: Arc<PlayerService>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            tracks,
            files,
            player,
            clock,
        }
    }

    pub async fn hide(&self, track_id: i64) -> CoreResult<()> {
        let inserted = self.tracks.hide(track_id, self.clock.now_ms()).await?;
        if let Err(error) = self.player.remove_track(track_id).await {
            if inserted {
                self.tracks.restore(track_id).await?;
            }
            return Err(error);
        }
        Ok(())
    }

    pub async fn restore(&self, track_id: i64) -> CoreResult<()> {
        self.tracks.restore(track_id).await
    }

    pub async fn hidden(&self) -> CoreResult<Vec<HiddenTrack>> {
        self.tracks.hidden().await
    }

    pub async fn delete_file(&self, track_id: i64) -> CoreResult<DeleteTrackResult> {
        let pending = self
            .tracks
            .begin_deletion(track_id, self.clock.now_ms())
            .await?;
        let outcome = match self.files.delete(&pending.uri).await {
            Ok(outcome) => outcome,
            Err(error) => {
                // Keep the native error stable even if best-effort cleanup has
                // its own failure. Recovery can reconcile the leftover row.
                let _ = self.tracks.cancel_deletion(track_id).await;
                return Err(error);
            }
        };

        match outcome {
            FileDeleteOutcome::Cancelled => {
                self.tracks.cancel_deletion(track_id).await?;
                Ok(DeleteTrackResult::Cancelled)
            }
            FileDeleteOutcome::Deleted => {
                self.tracks.mark_file_deleted(track_id).await?;
                self.player.remove_track(track_id).await?;
                self.tracks.finalize_deletion(track_id).await?;
                Ok(DeleteTrackResult::Deleted)
            }
        }
    }

    pub async fn recover_pending(&self) -> CoreResult<RecoveryReport> {
        let pending = self.tracks.pending_deletions().await?;
        let mut failures = Vec::new();
        let mut finalized_count = 0;
        for row in pending {
            match self.recover_one(&row).await {
                Ok(true) => finalized_count += 1,
                Ok(false) => {}
                Err(error) => failures.push((row.track_id, error)),
            }
        }
        Ok(RecoveryReport {
            finalized_count,
            failures,
        })
    }

    async fn recover_one(&self, pending: &PendingDeletion) -> CoreResult<bool> {
        if !pending.file_deleted {
            if self.files.exists(&pending.uri).await? {
                self.tracks.cancel_deletion(pending.track_id).await?;
                return Ok(false);
            }
            // Covers the crash gap between a successful native deletion and
            // persisting the recovery marker.
            self.tracks.mark_file_deleted(pending.track_id).await?;
        }

        self.player.remove_track(pending.track_id).await?;
        self.tracks.finalize_deletion(pending.track_id).await?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use super::{TrackFilePort, TrackRemovalService};
    use crate::application::player_service::PlayerPort;
    use crate::application::testing::{track, FixedClock};
    use crate::application::PlayerService;
    use crate::domain::{
        DeleteTrackResult, FileDeleteOutcome, HiddenTrack, LibraryStats, Page, PendingDeletion,
        PlaybackState, RepeatMode, ScannedTrack, Track, TrackQuery,
    };
    use crate::error::{CoreError, CoreResult};
    use crate::infrastructure::repositories::TrackRepository;

    const NOW: i64 = 1_723_700_000_000;

    #[derive(Clone, Copy)]
    enum DeleteBehavior {
        Deleted,
        Cancelled,
        Error,
    }

    #[derive(Clone, Copy)]
    enum ExistsBehavior {
        Exists(bool),
        Error,
    }

    struct FakeFiles {
        calls: Arc<Mutex<Vec<String>>>,
        delete_behavior: Mutex<DeleteBehavior>,
        exists: Mutex<HashMap<String, ExistsBehavior>>,
    }

    #[async_trait::async_trait]
    impl TrackFilePort for FakeFiles {
        async fn delete(&self, uri: &str) -> CoreResult<FileDeleteOutcome> {
            self.calls.lock().expect("lock").push("file_delete".into());
            match *self.delete_behavior.lock().expect("lock") {
                DeleteBehavior::Deleted => Ok(FileDeleteOutcome::Deleted),
                DeleteBehavior::Cancelled => Ok(FileDeleteOutcome::Cancelled),
                DeleteBehavior::Error => {
                    Err(CoreError::Player(format!("native delete failed for {uri}")))
                }
            }
        }

        async fn exists(&self, uri: &str) -> CoreResult<bool> {
            self.calls
                .lock()
                .expect("lock")
                .push(format!("exists:{uri}"));
            match self
                .exists
                .lock()
                .expect("lock")
                .get(uri)
                .copied()
                .unwrap_or(ExistsBehavior::Exists(true))
            {
                ExistsBehavior::Exists(exists) => Ok(exists),
                ExistsBehavior::Error => Err(CoreError::Player("provider probe failed".to_owned())),
            }
        }
    }

    struct FakeTracks {
        calls: Arc<Mutex<Vec<String>>>,
        pending: Mutex<Vec<PendingDeletion>>,
        hidden: Mutex<Vec<HiddenTrack>>,
        fail_finalize: Mutex<bool>,
    }

    impl FakeTracks {
        fn pending(&self) -> Vec<PendingDeletion> {
            self.pending.lock().expect("lock").clone()
        }
    }

    #[async_trait::async_trait]
    impl TrackRepository for FakeTracks {
        async fn get(&self, id: i64) -> CoreResult<Track> {
            Ok(track(id))
        }

        async fn get_many(&self, ids: &[i64]) -> CoreResult<Vec<Track>> {
            Ok(ids.iter().copied().map(track).collect())
        }

        async fn query(&self, query: &TrackQuery) -> CoreResult<Page<Track>> {
            Ok(Page::empty(query.offset, query.limit))
        }

        async fn recently_added(&self, _limit: i64) -> CoreResult<Vec<Track>> {
            Ok(Vec::new())
        }

        async fn hide(&self, track_id: i64, hidden_at: i64) -> CoreResult<bool> {
            self.calls.lock().expect("lock").push("hide".into());
            let mut hidden = self.hidden.lock().expect("lock");
            let inserted = !hidden.iter().any(|row| row.track.id == track_id);
            if inserted {
                hidden.push(HiddenTrack {
                    track: track(track_id),
                    hidden_at,
                });
            }
            Ok(inserted)
        }

        async fn restore(&self, track_id: i64) -> CoreResult<()> {
            self.calls.lock().expect("lock").push("restore".into());
            self.hidden
                .lock()
                .expect("lock")
                .retain(|hidden| hidden.track.id != track_id);
            Ok(())
        }

        async fn hidden(&self) -> CoreResult<Vec<HiddenTrack>> {
            Ok(self.hidden.lock().expect("lock").clone())
        }

        async fn begin_deletion(
            &self,
            track_id: i64,
            requested_at: i64,
        ) -> CoreResult<PendingDeletion> {
            self.calls.lock().expect("lock").push("begin".into());
            let pending = PendingDeletion {
                track_id,
                uri: track(track_id).uri,
                requested_at,
                file_deleted: false,
            };
            self.pending.lock().expect("lock").push(pending.clone());
            Ok(pending)
        }

        async fn cancel_deletion(&self, track_id: i64) -> CoreResult<()> {
            self.calls.lock().expect("lock").push("cancel".into());
            self.pending
                .lock()
                .expect("lock")
                .retain(|pending| pending.track_id != track_id);
            Ok(())
        }

        async fn mark_file_deleted(&self, track_id: i64) -> CoreResult<()> {
            self.calls.lock().expect("lock").push("mark".into());
            let mut pending = self.pending.lock().expect("lock");
            let row = pending
                .iter_mut()
                .find(|pending| pending.track_id == track_id)
                .ok_or_else(|| CoreError::not_found("pending deletion", track_id))?;
            row.file_deleted = true;
            Ok(())
        }

        async fn finalize_deletion(&self, track_id: i64) -> CoreResult<()> {
            self.calls.lock().expect("lock").push("finalize".into());
            if *self.fail_finalize.lock().expect("lock") {
                return Err(sqlx::Error::RowNotFound.into());
            }
            self.pending
                .lock()
                .expect("lock")
                .retain(|pending| pending.track_id != track_id);
            Ok(())
        }

        async fn pending_deletions(&self) -> CoreResult<Vec<PendingDeletion>> {
            Ok(self.pending())
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

    struct FakePlayer {
        calls: Arc<Mutex<Vec<String>>>,
        queue: Mutex<Vec<i64>>,
        fail_remove_at: Mutex<Option<i32>>,
    }

    #[async_trait::async_trait]
    impl PlayerPort for FakePlayer {
        async fn state(&self) -> CoreResult<PlaybackState> {
            Ok(PlaybackState::idle())
        }

        async fn queue_ids(&self) -> CoreResult<Vec<i64>> {
            Ok(self.queue.lock().expect("lock").clone())
        }

        async fn set_queue(
            &self,
            _tracks: &[Track],
            _start_index: i32,
            _autoplay: bool,
        ) -> CoreResult<()> {
            Ok(())
        }

        async fn add_next(&self, _tracks: &[Track]) -> CoreResult<()> {
            Ok(())
        }

        async fn add_to_queue(&self, _tracks: &[Track]) -> CoreResult<()> {
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

        async fn set_crossfade(&self, _duration_ms: i64) -> CoreResult<()> {
            Ok(())
        }

        async fn remove_queue_item(&self, index: i32) -> CoreResult<()> {
            self.calls.lock().expect("lock").push("queue_remove".into());
            let mut queue = self.queue.lock().expect("lock");
            if let Ok(index) = usize::try_from(index) {
                if index < queue.len() {
                    queue.remove(index);
                }
            }
            if *self.fail_remove_at.lock().expect("lock") == Some(index) {
                return Err(CoreError::Player("partial queue removal".to_owned()));
            }
            Ok(())
        }

        async fn move_queue_item(&self, _from: i32, _to: i32) -> CoreResult<()> {
            Ok(())
        }

        async fn clear_queue(&self) -> CoreResult<()> {
            Ok(())
        }
    }

    struct Harness {
        service: TrackRemovalService,
        tracks: Arc<FakeTracks>,
        files: Arc<FakeFiles>,
        player: Arc<FakePlayer>,
        player_service: Arc<PlayerService>,
        calls: Arc<Mutex<Vec<String>>>,
    }

    fn harness(queue: Vec<i64>, behavior: DeleteBehavior) -> Harness {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let tracks = Arc::new(FakeTracks {
            calls: calls.clone(),
            pending: Mutex::new(Vec::new()),
            hidden: Mutex::new(Vec::new()),
            fail_finalize: Mutex::new(false),
        });
        let files = Arc::new(FakeFiles {
            calls: calls.clone(),
            delete_behavior: Mutex::new(behavior),
            exists: Mutex::new(HashMap::new()),
        });
        let player = Arc::new(FakePlayer {
            calls: calls.clone(),
            queue: Mutex::new(queue),
            fail_remove_at: Mutex::new(None),
        });
        let player_service = Arc::new(PlayerService::new(tracks.clone(), player.clone()));
        let service = TrackRemovalService::with_clock(
            tracks.clone(),
            files.clone(),
            player_service.clone(),
            Arc::new(FixedClock(NOW)),
        );
        Harness {
            service,
            tracks,
            files,
            player,
            player_service,
            calls,
        }
    }

    #[tokio::test]
    async fn hide_writes_the_tombstone_before_removing_the_queue_item() {
        let harness = harness(vec![1], DeleteBehavior::Deleted);

        harness.service.hide(1).await.expect("hidden");

        assert_eq!(
            *harness.calls.lock().expect("lock"),
            vec!["hide", "queue_remove"]
        );
        let hidden = harness.service.hidden().await.expect("hidden list");
        assert_eq!(hidden[0].hidden_at, NOW);
    }

    #[tokio::test]
    async fn restore_only_removes_the_tombstone() {
        let harness = harness(Vec::new(), DeleteBehavior::Deleted);
        harness.service.hide(1).await.expect("hidden");
        harness.calls.lock().expect("lock").clear();

        harness.service.restore(1).await.expect("restored");

        assert_eq!(*harness.calls.lock().expect("lock"), vec!["restore"]);
        assert!(harness
            .service
            .hidden()
            .await
            .expect("hidden list")
            .is_empty());
    }

    #[tokio::test]
    async fn hide_rolls_back_the_tombstone_after_a_partial_queue_failure() {
        let harness = harness(vec![1, 1, 1], DeleteBehavior::Deleted);
        *harness.player.fail_remove_at.lock().expect("lock") = Some(1);

        let error = harness.service.hide(1).await.expect_err("queue failure");

        assert_eq!(error.code(), "PLAYER");
        assert!(harness
            .service
            .hidden()
            .await
            .expect("hidden list")
            .is_empty());
        assert_eq!(*harness.player.queue.lock().expect("lock"), vec![1]);
        assert_eq!(harness.player_service.queue_snapshot(), vec![1]);
        assert_eq!(
            *harness.calls.lock().expect("lock"),
            vec!["hide", "queue_remove", "queue_remove", "restore"]
        );
    }

    #[tokio::test]
    async fn retrying_an_already_hidden_track_does_not_rollback_its_tombstone() {
        let harness = harness(vec![1, 1], DeleteBehavior::Deleted);
        harness
            .tracks
            .hidden
            .lock()
            .expect("lock")
            .push(HiddenTrack {
                track: track(1),
                hidden_at: NOW - 1,
            });
        *harness.player.fail_remove_at.lock().expect("lock") = Some(1);

        let error = harness.service.hide(1).await.expect_err("queue failure");

        assert_eq!(error.code(), "PLAYER");
        assert_eq!(harness.service.hidden().await.expect("hidden").len(), 1);
        assert_eq!(
            *harness.calls.lock().expect("lock"),
            vec!["hide", "queue_remove"]
        );
    }

    #[tokio::test]
    async fn delete_success_records_the_crash_boundary_before_cleanup() {
        let harness = harness(vec![1], DeleteBehavior::Deleted);

        let result = harness.service.delete_file(1).await.expect("deleted");

        assert_eq!(result, DeleteTrackResult::Deleted);
        assert_eq!(
            *harness.calls.lock().expect("lock"),
            vec!["begin", "file_delete", "mark", "queue_remove", "finalize"]
        );
        assert!(harness.tracks.pending().is_empty());
    }

    #[tokio::test]
    async fn cancellation_is_normal_and_clears_the_pending_operation() {
        let harness = harness(vec![1], DeleteBehavior::Cancelled);

        let result = harness.service.delete_file(1).await.expect("cancelled");

        assert_eq!(result, DeleteTrackResult::Cancelled);
        assert_eq!(
            *harness.calls.lock().expect("lock"),
            vec!["begin", "file_delete", "cancel"]
        );
        assert!(harness.tracks.pending().is_empty());
        assert_eq!(*harness.player.queue.lock().expect("lock"), vec![1]);
    }

    #[tokio::test]
    async fn native_failure_clears_pending_and_preserves_the_native_error() {
        let harness = harness(vec![1], DeleteBehavior::Error);

        let error = harness
            .service
            .delete_file(1)
            .await
            .expect_err("native failure");

        assert_eq!(error.code(), "PLAYER");
        assert!(error.to_string().contains("native delete failed"));
        assert_eq!(
            *harness.calls.lock().expect("lock"),
            vec!["begin", "file_delete", "cancel"]
        );
        assert!(harness.tracks.pending().is_empty());
    }

    #[tokio::test]
    async fn database_failure_after_native_success_leaves_a_recoverable_marker() {
        let harness = harness(vec![1], DeleteBehavior::Deleted);
        *harness.tracks.fail_finalize.lock().expect("lock") = true;

        let error = harness
            .service
            .delete_file(1)
            .await
            .expect_err("finalize failure");

        assert_eq!(error.code(), "DATABASE");
        assert!(harness.tracks.pending()[0].file_deleted);
        assert_eq!(
            *harness.calls.lock().expect("lock"),
            vec!["begin", "file_delete", "mark", "queue_remove", "finalize"]
        );
    }

    #[tokio::test]
    async fn recovery_handles_marked_missing_existing_and_indeterminate_rows() {
        let harness = harness(vec![1, 2, 3, 4], DeleteBehavior::Deleted);
        let uri = |id| track(id).uri;
        *harness.tracks.pending.lock().expect("lock") = vec![
            PendingDeletion {
                track_id: 1,
                uri: uri(1),
                requested_at: NOW - 4,
                file_deleted: true,
            },
            PendingDeletion {
                track_id: 2,
                uri: uri(2),
                requested_at: NOW - 3,
                file_deleted: false,
            },
            PendingDeletion {
                track_id: 3,
                uri: uri(3),
                requested_at: NOW - 2,
                file_deleted: false,
            },
            PendingDeletion {
                track_id: 4,
                uri: uri(4),
                requested_at: NOW - 1,
                file_deleted: false,
            },
        ];
        harness.files.exists.lock().expect("lock").extend([
            (uri(2), ExistsBehavior::Exists(false)),
            (uri(3), ExistsBehavior::Exists(true)),
            (uri(4), ExistsBehavior::Error),
        ]);

        let report = harness
            .service
            .recover_pending()
            .await
            .expect("recovery runs");

        assert_eq!(report.finalized_count, 2);
        assert_eq!(report.failures.len(), 1);
        assert_eq!(report.failures[0].0, 4);
        assert_eq!(report.failures[0].1.code(), "PLAYER");
        assert_eq!(
            harness.tracks.pending(),
            vec![PendingDeletion {
                track_id: 4,
                uri: uri(4),
                requested_at: NOW - 1,
                file_deleted: false,
            }]
        );
        assert_eq!(*harness.player.queue.lock().expect("lock"), vec![3, 4]);
    }

    #[tokio::test]
    async fn recovery_reports_no_library_change_when_nothing_is_pending() {
        let harness = harness(Vec::new(), DeleteBehavior::Deleted);

        let report = harness
            .service
            .recover_pending()
            .await
            .expect("recovery runs");

        assert_eq!(report.finalized_count, 0);
        assert!(report.failures.is_empty());
    }
}
