//! Library scanning (CONTRACTS §5, `library_scan*`, `library_roots`,
//! `library_pick_folder`, `library_remove_root`).
//!
//! The native side enumerates MediaStore / SAF trees and hands back batches of
//! [`ScannedTrack`]; this service drives that loop, writes each batch through
//! the repository and keeps a [`ScanStatus`] the command layer can both read
//! (`library_scan_status`) and forward as `library:scan-progress` (§6).

use std::sync::{Arc, Mutex, MutexGuard};

use crate::application::{count_as_i64, Clock, SystemClock};
use crate::domain::{
    ScanMode, ScanResult, ScanStatus, ScannedTrack, PHASE_CLEANUP, PHASE_DONE, PHASE_ENUMERATING,
    PHASE_IDLE, PHASE_READING, PHASE_WRITING,
};
use crate::error::{CoreError, CoreResult};
use crate::infrastructure::repositories::TrackRepository;

/// Root value that means "everything MediaStore knows about", as opposed to a
/// SAF tree URI. An empty root list means the same thing.
pub const MEDIA_STORE_ROOT: &str = "mediastore";

/// Runaway guard: a native scanner that keeps handing out fresh cursors would
/// otherwise loop forever.
const MAX_BATCHES: usize = 10_000;

/// Called with a fresh snapshot every time the scan advances. The command layer
/// turns it into the `library:scan-progress` event.
pub type ProgressCallback = Box<dyn Fn(ScanStatus) + Send + Sync>;

/// One page of a scan. `complete == false` means "call again, passing
/// `next_cursor` back".
#[derive(Debug, Clone, PartialEq)]
pub struct ScanBatch {
    pub tracks: Vec<ScannedTrack>,
    pub complete: bool,
    pub next_cursor: Option<String>,
}

impl ScanBatch {
    /// The last page of a scan.
    pub fn complete(tracks: Vec<ScannedTrack>) -> Self {
        Self {
            tracks,
            complete: true,
            next_cursor: None,
        }
    }

    /// A page with more to follow.
    pub fn partial(tracks: Vec<ScannedTrack>, next_cursor: impl Into<String>) -> Self {
        Self {
            tracks,
            complete: false,
            next_cursor: Some(next_cursor.into()),
        }
    }

    /// A finished scan that found nothing.
    pub fn empty() -> Self {
        Self::complete(Vec::new())
    }
}

/// The native scanner and the file-access side of the plugin (CONTRACTS §7.1),
/// with the plugin types replaced by domain types.
///
/// `cursor` carries [`ScanBatch::next_cursor`] back to the native side so a
/// large library can be read in pages.
#[async_trait::async_trait]
pub trait ScannerPort: Send + Sync {
    async fn scan_media_store(
        &self,
        since: Option<i64>,
        cursor: Option<&str>,
    ) -> CoreResult<ScanBatch>;
    async fn scan_tree(
        &self,
        tree_uri: &str,
        since: Option<i64>,
        cursor: Option<&str>,
    ) -> CoreResult<ScanBatch>;
    /// `ACTION_OPEN_DOCUMENT_TREE`; `None` when the user cancelled.
    async fn pick_folder(&self) -> CoreResult<Option<String>>;
    async fn persisted_roots(&self) -> CoreResult<Vec<String>>;
    async fn release_root(&self, tree_uri: &str) -> CoreResult<()>;
    async fn extract_artwork(&self, uri: &str) -> CoreResult<Option<String>>;
}

/// What to enumerate.
enum Source<'a> {
    MediaStore,
    Tree(&'a str),
}

#[derive(Debug, Clone)]
struct ScanState {
    status: ScanStatus,
    /// Start of the last successful scan, used as the `since` cursor of the
    /// next incremental one.
    last_scan_ms: Option<i64>,
}

pub struct ScanService {
    scanner: Arc<dyn ScannerPort>,
    tracks: Arc<dyn TrackRepository>,
    clock: Arc<dyn Clock>,
    progress: Option<ProgressCallback>,
    state: Mutex<ScanState>,
}

impl ScanService {
    pub fn new(scanner: Arc<dyn ScannerPort>, tracks: Arc<dyn TrackRepository>) -> Self {
        Self {
            scanner,
            tracks,
            clock: Arc::new(SystemClock),
            progress: None,
            state: Mutex::new(ScanState {
                status: ScanStatus::idle(),
                last_scan_ms: None,
            }),
        }
    }

    pub fn with_progress(mut self, progress: ProgressCallback) -> Self {
        self.progress = Some(progress);
        self
    }

    pub fn with_clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }

    /// Timestamp a restarted app already knows about, so the first incremental
    /// scan of a session does not re-read the whole library.
    pub fn seed_last_scan(&self, last_scan_ms: Option<i64>) {
        self.state().last_scan_ms = last_scan_ms;
    }

    pub fn status(&self) -> ScanStatus {
        self.state().status.clone()
    }

    /// Progress reported by the native scanner (`scanProgress`, §7.2). It is
    /// the only source of a meaningful `total`, so it wins over the batch
    /// counter — but only while a scan of ours is actually running.
    pub fn report_native_progress(&self, scanned: i64, total: i64, phase: &str) {
        {
            let mut state = self.state();
            if !state.status.running {
                return;
            }
            state.status.scanned = state.status.scanned.max(scanned.max(0));
            state.status.total = state.status.total.max(total.max(0));
            if !phase.trim().is_empty() {
                state.status.phase = phase.to_owned();
            }
        }
        self.publish();
    }

    /// Full or incremental scan of `roots`. An empty list scans MediaStore.
    /// Only one scan runs at a time.
    pub async fn scan(&self, roots: &[String], mode: ScanMode) -> CoreResult<ScanResult> {
        self.begin()?;
        let outcome = self.run(roots, mode).await;
        self.finish(outcome.is_ok());
        outcome
    }

    pub async fn pick_folder(&self) -> CoreResult<Option<String>> {
        self.scanner.pick_folder().await
    }

    pub async fn roots(&self) -> CoreResult<Vec<String>> {
        self.scanner.persisted_roots().await
    }

    pub async fn remove_root(&self, tree_uri: &str) -> CoreResult<()> {
        let uri = tree_uri.trim();
        if uri.is_empty() {
            return Err(CoreError::invalid_input("uri must not be blank"));
        }
        self.scanner.release_root(uri).await
    }

    /// Embedded cover of a single file, extracted into the app cache.
    pub async fn extract_artwork(&self, uri: &str) -> CoreResult<Option<String>> {
        let uri = uri.trim();
        if uri.is_empty() {
            return Err(CoreError::invalid_input("uri must not be blank"));
        }
        self.scanner.extract_artwork(uri).await
    }

    async fn run(&self, roots: &[String], mode: ScanMode) -> CoreResult<ScanResult> {
        let started_at = self.clock.now_ms();
        let since = mode.since(self.state().last_scan_ms);
        let before = self.tracks.stats().await?.tracks;

        let mut seen_uris: Vec<String> = Vec::new();
        let mut written: u64 = 0;

        for source in sources(roots) {
            let mut cursor: Option<String> = None;
            let mut batches = 0usize;
            loop {
                self.set_phase(PHASE_READING);
                let batch = self.fetch(&source, since, cursor.as_deref()).await?;

                if !batch.tracks.is_empty() {
                    self.set_phase(PHASE_WRITING);
                    seen_uris.extend(batch.tracks.iter().map(|track| track.uri.clone()));
                    written = written.saturating_add(self.tracks.upsert_many(&batch.tracks).await?);
                    self.advance(batch.tracks.len());
                }

                match continuation(&batch, cursor.as_deref()) {
                    Some(next) => cursor = Some(next),
                    None => break,
                }

                batches += 1;
                if batches >= MAX_BATCHES {
                    return Err(CoreError::Scan(format!(
                        "scanner did not finish after {MAX_BATCHES} batches"
                    )));
                }
            }
        }

        // `upsert_many` reports rows written without telling inserts from
        // updates, so the split comes from the track count around the write.
        let after = self.tracks.stats().await?.tracks;
        let added = after.saturating_sub(before).max(0);
        let updated = count_as_i64(written).saturating_sub(added).max(0);

        let mut removed = 0;
        // A full scan that found nothing means the roots went away or a
        // permission was revoked — pruning there would wipe the library.
        if !mode.is_incremental() && !seen_uris.is_empty() {
            self.set_phase(PHASE_CLEANUP);
            removed = count_as_i64(self.tracks.delete_missing(&seen_uris).await?);
        }

        self.state().last_scan_ms = Some(started_at);
        Ok(ScanResult {
            added,
            updated,
            removed,
            duration_ms: self.clock.now_ms().saturating_sub(started_at).max(0),
        })
    }

    async fn fetch(
        &self,
        source: &Source<'_>,
        since: Option<i64>,
        cursor: Option<&str>,
    ) -> CoreResult<ScanBatch> {
        match source {
            Source::MediaStore => self.scanner.scan_media_store(since, cursor).await,
            Source::Tree(tree_uri) => self.scanner.scan_tree(tree_uri, since, cursor).await,
        }
    }

    /// Claims the "running" flag; the claim and the check are one critical
    /// section, so two concurrent commands cannot both win.
    fn begin(&self) -> CoreResult<()> {
        {
            let mut state = self.state();
            if state.status.running {
                return Err(CoreError::invalid_input("a scan is already running"));
            }
            state.status = ScanStatus::in_progress(PHASE_ENUMERATING, 0, 0);
        }
        self.publish();
        Ok(())
    }

    fn finish(&self, succeeded: bool) {
        let phase = if succeeded { PHASE_DONE } else { PHASE_IDLE };
        {
            let mut state = self.state();
            state.status.running = false;
            state.status.phase = phase.to_owned();
            if succeeded {
                state.status.total = state.status.total.max(state.status.scanned);
            }
        }
        self.publish();
    }

    fn set_phase(&self, phase: &str) {
        {
            let mut state = self.state();
            if state.status.phase == phase {
                return;
            }
            state.status.phase = phase.to_owned();
        }
        self.publish();
    }

    fn advance(&self, count: usize) {
        {
            let mut state = self.state();
            let scanned = state
                .status
                .scanned
                .saturating_add(count_as_i64(count as u64));
            state.status.scanned = scanned;
            state.status.total = state.status.total.max(scanned);
        }
        self.publish();
    }

    fn publish(&self) {
        if let Some(progress) = &self.progress {
            let snapshot = self.status();
            progress(snapshot);
        }
    }

    fn state(&self) -> MutexGuard<'_, ScanState> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// Blank roots are dropped and duplicates collapsed; nothing left means "scan
/// MediaStore".
fn sources(roots: &[String]) -> Vec<Source<'_>> {
    let mut trees: Vec<&str> = Vec::new();
    let mut media_store = false;

    for root in roots {
        let root = root.trim();
        if root.is_empty() {
            continue;
        }
        if root.eq_ignore_ascii_case(MEDIA_STORE_ROOT) {
            media_store = true;
        } else if !trees.contains(&root) {
            trees.push(root);
        }
    }

    if trees.is_empty() {
        return vec![Source::MediaStore];
    }

    let mut sources: Vec<Source<'_>> = Vec::with_capacity(trees.len() + usize::from(media_store));
    if media_store {
        sources.push(Source::MediaStore);
    }
    sources.extend(trees.into_iter().map(Source::Tree));
    sources
}

/// Cursor of the next page, or `None` when the scan of this source is over.
/// A repeated or missing cursor counts as "over", so a native side that never
/// sets `complete` still terminates.
fn continuation(batch: &ScanBatch, previous: Option<&str>) -> Option<String> {
    if batch.complete {
        return None;
    }
    let next = batch.next_cursor.as_deref()?.trim();
    if next.is_empty() || Some(next) == previous {
        return None;
    }
    Some(next.to_owned())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::sync::{Arc, Mutex};

    use tokio::sync::Notify;

    use super::{continuation, ScanBatch, ScanService, ScannerPort, MEDIA_STORE_ROOT};
    use crate::application::Clock;
    use crate::domain::{
        LibraryStats, Page, ScanMode, ScanResult, ScannedTrack, Track, TrackQuery, PHASE_DONE,
        PHASE_ENUMERATING, PHASE_IDLE,
    };
    use crate::error::{CoreError, CoreResult};
    use crate::infrastructure::repositories::TrackRepository;

    const NOW: i64 = 1_700_000_000_000;

    /// Advances a second on every read, so a scan has a measurable duration.
    struct SteppingClock(Mutex<i64>);

    impl Clock for SteppingClock {
        fn now_ms(&self) -> i64 {
            let mut now = self.0.lock().expect("lock");
            let current = *now;
            *now += 1_000;
            current
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    enum ScanCall {
        MediaStore(Option<i64>, Option<String>),
        Tree(String, Option<i64>, Option<String>),
    }

    #[derive(Default)]
    struct FakeScanner {
        calls: Mutex<Vec<ScanCall>>,
        batches: Mutex<Vec<ScanBatch>>,
        roots: Mutex<Vec<String>>,
        released: Mutex<Vec<String>>,
        picked: Mutex<Option<String>>,
    }

    impl FakeScanner {
        fn with_batches(batches: Vec<ScanBatch>) -> Self {
            Self {
                batches: Mutex::new(batches),
                ..Self::default()
            }
        }

        fn calls(&self) -> Vec<ScanCall> {
            self.calls.lock().expect("lock").clone()
        }

        fn next_batch(&self) -> ScanBatch {
            let mut batches = self.batches.lock().expect("lock");
            if batches.is_empty() {
                return ScanBatch::empty();
            }
            batches.remove(0)
        }
    }

    #[async_trait::async_trait]
    impl ScannerPort for FakeScanner {
        async fn scan_media_store(
            &self,
            since: Option<i64>,
            cursor: Option<&str>,
        ) -> CoreResult<ScanBatch> {
            self.calls
                .lock()
                .expect("lock")
                .push(ScanCall::MediaStore(since, cursor.map(str::to_owned)));
            Ok(self.next_batch())
        }

        async fn scan_tree(
            &self,
            tree_uri: &str,
            since: Option<i64>,
            cursor: Option<&str>,
        ) -> CoreResult<ScanBatch> {
            self.calls.lock().expect("lock").push(ScanCall::Tree(
                tree_uri.to_owned(),
                since,
                cursor.map(str::to_owned),
            ));
            Ok(self.next_batch())
        }

        async fn pick_folder(&self) -> CoreResult<Option<String>> {
            Ok(self.picked.lock().expect("lock").clone())
        }

        async fn persisted_roots(&self) -> CoreResult<Vec<String>> {
            Ok(self.roots.lock().expect("lock").clone())
        }

        async fn release_root(&self, tree_uri: &str) -> CoreResult<()> {
            self.released
                .lock()
                .expect("lock")
                .push(tree_uri.to_owned());
            Ok(())
        }

        async fn extract_artwork(&self, uri: &str) -> CoreResult<Option<String>> {
            Ok(Some(format!("file:///cache/{uri}.jpg")))
        }
    }

    /// Blocks inside the first scan call until released.
    struct BlockingScanner {
        entered: Arc<Notify>,
        release: Arc<Notify>,
    }

    #[async_trait::async_trait]
    impl ScannerPort for BlockingScanner {
        async fn scan_media_store(
            &self,
            _since: Option<i64>,
            _cursor: Option<&str>,
        ) -> CoreResult<ScanBatch> {
            self.entered.notify_one();
            self.release.notified().await;
            Ok(ScanBatch::empty())
        }

        async fn scan_tree(
            &self,
            _tree_uri: &str,
            _since: Option<i64>,
            _cursor: Option<&str>,
        ) -> CoreResult<ScanBatch> {
            Ok(ScanBatch::empty())
        }

        async fn pick_folder(&self) -> CoreResult<Option<String>> {
            Ok(None)
        }

        async fn persisted_roots(&self) -> CoreResult<Vec<String>> {
            Ok(Vec::new())
        }

        async fn release_root(&self, _tree_uri: &str) -> CoreResult<()> {
            Ok(())
        }

        async fn extract_artwork(&self, _uri: &str) -> CoreResult<Option<String>> {
            Ok(None)
        }
    }

    /// Tracks are identified by URI, exactly like the real table.
    #[derive(Default)]
    struct FakeTracks {
        known: Mutex<BTreeSet<String>>,
        upserts: Mutex<Vec<Vec<String>>>,
        pruned: Mutex<Vec<Vec<String>>>,
    }

    impl FakeTracks {
        fn seeded(uris: &[&str]) -> Self {
            Self {
                known: Mutex::new(uris.iter().map(|uri| (*uri).to_owned()).collect()),
                ..Self::default()
            }
        }
    }

    #[async_trait::async_trait]
    impl TrackRepository for FakeTracks {
        async fn get(&self, id: i64) -> CoreResult<Track> {
            Err(CoreError::not_found("track", id))
        }

        async fn get_many(&self, _ids: &[i64]) -> CoreResult<Vec<Track>> {
            Ok(Vec::new())
        }

        async fn query(&self, q: &TrackQuery) -> CoreResult<Page<Track>> {
            Ok(Page::empty(q.offset, q.limit))
        }

        async fn recently_added(&self, _limit: i64) -> CoreResult<Vec<Track>> {
            Ok(Vec::new())
        }

        async fn upsert_many(&self, tracks: &[ScannedTrack]) -> CoreResult<u64> {
            let uris: Vec<String> = tracks.iter().map(|track| track.uri.clone()).collect();
            self.known.lock().expect("lock").extend(uris.clone());
            self.upserts.lock().expect("lock").push(uris);
            Ok(tracks.len() as u64)
        }

        async fn delete_missing(&self, keep_uris: &[String]) -> CoreResult<u64> {
            self.pruned.lock().expect("lock").push(keep_uris.to_vec());
            let keep: BTreeSet<&String> = keep_uris.iter().collect();
            let mut known = self.known.lock().expect("lock");
            let before = known.len();
            known.retain(|uri| keep.contains(uri));
            Ok((before - known.len()) as u64)
        }

        async fn stats(&self) -> CoreResult<LibraryStats> {
            Ok(LibraryStats {
                tracks: self.known.lock().expect("lock").len() as i64,
                ..LibraryStats::default()
            })
        }
    }

    fn scanned(uri: &str) -> ScannedTrack {
        ScannedTrack {
            uri: uri.to_owned(),
            title: "Title".to_owned(),
            artist: Some("Artist".to_owned()),
            album: Some("Album".to_owned()),
            album_artist: None,
            duration_ms: 200_000,
            track_number: Some(1),
            disc_number: Some(1),
            year: Some(2020),
            genre: Some("Rock".to_owned()),
            bitrate: Some(320_000),
            sample_rate: Some(44_100),
            size: 8_000_000,
            mime_type: Some("audio/mpeg".to_owned()),
            folder: Some("Music".to_owned()),
            date_added: NOW,
            last_modified: NOW,
            cover_key: None,
        }
    }

    fn service(scanner: Arc<FakeScanner>, tracks: Arc<FakeTracks>) -> ScanService {
        ScanService::new(scanner, tracks).with_clock(Arc::new(SteppingClock(Mutex::new(NOW))))
    }

    #[tokio::test]
    async fn full_scan_writes_every_batch_and_prunes_what_disappeared() {
        let scanner = Arc::new(FakeScanner::with_batches(vec![ScanBatch::complete(vec![
            scanned("content://old"),
            scanned("content://new"),
        ])]));
        let tracks = Arc::new(FakeTracks::seeded(&["content://old", "content://gone"]));
        let service = service(scanner.clone(), tracks.clone());

        let result = service.scan(&[], ScanMode::Full).await.expect("scan");

        assert_eq!(result.added, 1, "only content://new is new");
        assert_eq!(result.updated, 1, "content://old was rewritten");
        assert_eq!(result.removed, 1, "content://gone is no longer on disk");
        assert_eq!(result.duration_ms, 1_000);
        assert_eq!(
            scanner.calls(),
            vec![ScanCall::MediaStore(None, None)],
            "an empty root list scans MediaStore"
        );
        assert_eq!(
            tracks.pruned.lock().expect("lock").as_slice(),
            &[vec!["content://old".to_owned(), "content://new".to_owned()]]
        );
    }

    #[tokio::test]
    async fn batches_are_pulled_until_the_scanner_says_it_is_done() {
        let scanner = Arc::new(FakeScanner::with_batches(vec![
            ScanBatch::partial(vec![scanned("content://1")], "cursor-1"),
            ScanBatch::partial(vec![scanned("content://2")], "cursor-2"),
            ScanBatch::complete(vec![scanned("content://3")]),
        ]));
        let tracks = Arc::new(FakeTracks::default());
        let service = service(scanner.clone(), tracks.clone());

        let result = service.scan(&[], ScanMode::Full).await.expect("scan");

        assert_eq!(result.added, 3);
        assert_eq!(result.updated, 0);
        assert_eq!(
            scanner.calls(),
            vec![
                ScanCall::MediaStore(None, None),
                ScanCall::MediaStore(None, Some("cursor-1".to_owned())),
                ScanCall::MediaStore(None, Some("cursor-2".to_owned())),
            ]
        );
        assert_eq!(tracks.upserts.lock().expect("lock").len(), 3);
        assert_eq!(service.status().scanned, 3);
    }

    #[tokio::test]
    async fn an_unfinished_batch_without_a_fresh_cursor_stops_the_loop() {
        let stuck = ScanBatch {
            tracks: vec![scanned("content://1")],
            complete: false,
            next_cursor: None,
        };
        let scanner = Arc::new(FakeScanner::with_batches(vec![stuck]));
        let service = service(scanner.clone(), Arc::new(FakeTracks::default()));

        service.scan(&[], ScanMode::Full).await.expect("scan");

        assert_eq!(scanner.calls().len(), 1);
        assert_eq!(
            continuation(&ScanBatch::partial(Vec::new(), "same"), Some("same")),
            None,
            "a repeated cursor is a finished scan"
        );
        assert_eq!(
            continuation(&ScanBatch::partial(Vec::new(), " next "), Some("same")),
            Some("next".to_owned())
        );
    }

    #[tokio::test]
    async fn incremental_scan_reuses_the_previous_timestamp_and_never_prunes() {
        let scanner = Arc::new(FakeScanner::with_batches(vec![
            ScanBatch::complete(vec![scanned("content://1")]),
            ScanBatch::complete(vec![scanned("content://1")]),
        ]));
        let tracks = Arc::new(FakeTracks::default());
        let service = service(scanner.clone(), tracks.clone());

        service.scan(&[], ScanMode::Full).await.expect("full scan");
        let result = service
            .scan(&[], ScanMode::Incremental)
            .await
            .expect("incremental scan");

        assert_eq!(result.added, 0);
        assert_eq!(result.updated, 1);
        assert_eq!(result.removed, 0);
        assert_eq!(
            scanner.calls(),
            vec![
                ScanCall::MediaStore(None, None),
                ScanCall::MediaStore(Some(NOW), None),
            ],
            "the incremental pass asks for changes since the last scan started"
        );
        assert_eq!(
            tracks.pruned.lock().expect("lock").len(),
            1,
            "only the full scan prunes"
        );
    }

    #[tokio::test]
    async fn a_full_scan_that_found_nothing_leaves_the_library_alone() {
        let tracks = Arc::new(FakeTracks::seeded(&["content://old"]));
        let service = service(Arc::new(FakeScanner::default()), tracks.clone());

        let result = service.scan(&[], ScanMode::Full).await.expect("scan");

        assert_eq!(
            result,
            ScanResult {
                added: 0,
                updated: 0,
                removed: 0,
                duration_ms: 1_000,
            }
        );
        assert!(tracks.pruned.lock().expect("lock").is_empty());
        assert_eq!(tracks.known.lock().expect("lock").len(), 1);
    }

    #[tokio::test]
    async fn every_root_is_scanned_once_and_blank_ones_are_dropped() {
        let scanner = Arc::new(FakeScanner::default());
        let service = service(scanner.clone(), Arc::new(FakeTracks::default()));

        let roots = vec![
            "  content://tree/a ".to_owned(),
            "content://tree/a".to_owned(),
            "   ".to_owned(),
            MEDIA_STORE_ROOT.to_owned(),
            "content://tree/b".to_owned(),
        ];
        service.scan(&roots, ScanMode::Full).await.expect("scan");

        assert_eq!(
            scanner.calls(),
            vec![
                ScanCall::MediaStore(None, None),
                ScanCall::Tree("content://tree/a".to_owned(), None, None),
                ScanCall::Tree("content://tree/b".to_owned(), None, None),
            ]
        );
    }

    #[tokio::test]
    async fn a_second_scan_is_rejected_while_one_is_running() {
        let entered = Arc::new(Notify::new());
        let release = Arc::new(Notify::new());
        let scanner = Arc::new(BlockingScanner {
            entered: entered.clone(),
            release: release.clone(),
        });
        let service = Arc::new(ScanService::new(scanner, Arc::new(FakeTracks::default())));

        let running = {
            let service = service.clone();
            tokio::spawn(async move { service.scan(&[], ScanMode::Full).await })
        };
        entered.notified().await;

        assert!(service.status().running);
        let rejected = service
            .scan(&[], ScanMode::Full)
            .await
            .expect_err("a scan is already running");
        assert_eq!(rejected.code(), "INVALID_INPUT");

        release.notify_one();
        running.await.expect("join").expect("first scan finishes");

        assert!(!service.status().running);
        // One permit per scan: this one is waiting before the scan asks for it.
        release.notify_one();
        service
            .scan(&[], ScanMode::Full)
            .await
            .expect("the flag is released once the scan is over");
    }

    #[tokio::test]
    async fn progress_is_published_and_ends_on_a_final_snapshot() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let sink = seen.clone();
        let scanner = Arc::new(FakeScanner::with_batches(vec![
            ScanBatch::partial(vec![scanned("content://1")], "cursor-1"),
            ScanBatch::complete(vec![scanned("content://2")]),
        ]));
        let service = service(scanner, Arc::new(FakeTracks::default())).with_progress(Box::new(
            move |status| sink.lock().expect("lock").push(status),
        ));

        service.scan(&[], ScanMode::Full).await.expect("scan");

        let statuses = seen.lock().expect("lock").clone();
        let first = statuses.first().expect("a scan publishes its start");
        assert!(first.running);
        assert_eq!(first.phase, PHASE_ENUMERATING);
        assert_eq!(first.scanned, 0);

        let last = statuses.last().expect("a scan publishes its end");
        assert!(!last.running);
        assert_eq!(last.phase, PHASE_DONE);
        assert_eq!(last.scanned, 2);
        assert_eq!(last.total, 2);
        assert_eq!(&service.status(), last);
        assert!(
            statuses.iter().any(|status| status.scanned == 1),
            "progress is reported per batch, not only at the end"
        );
    }

    #[tokio::test]
    async fn a_failed_scan_releases_the_flag_and_reports_idle() {
        struct FailingTracks;

        #[async_trait::async_trait]
        impl TrackRepository for FailingTracks {
            async fn get(&self, id: i64) -> CoreResult<Track> {
                Err(CoreError::not_found("track", id))
            }

            async fn get_many(&self, _ids: &[i64]) -> CoreResult<Vec<Track>> {
                Ok(Vec::new())
            }

            async fn query(&self, q: &TrackQuery) -> CoreResult<Page<Track>> {
                Ok(Page::empty(q.offset, q.limit))
            }

            async fn recently_added(&self, _limit: i64) -> CoreResult<Vec<Track>> {
                Ok(Vec::new())
            }

            async fn upsert_many(&self, _tracks: &[ScannedTrack]) -> CoreResult<u64> {
                Err(CoreError::Database(sqlx::Error::RowNotFound))
            }

            async fn delete_missing(&self, _keep_uris: &[String]) -> CoreResult<u64> {
                Ok(0)
            }

            async fn stats(&self) -> CoreResult<LibraryStats> {
                Ok(LibraryStats::default())
            }
        }

        let scanner = Arc::new(FakeScanner::with_batches(vec![ScanBatch::complete(vec![
            scanned("content://1"),
        ])]));
        let service = ScanService::new(scanner, Arc::new(FailingTracks));

        let error = service
            .scan(&[], ScanMode::Full)
            .await
            .expect_err("the write failed");

        assert_eq!(error.code(), "DATABASE");
        let status = service.status();
        assert!(!status.running);
        assert_eq!(status.phase, PHASE_IDLE);
    }

    #[tokio::test]
    async fn native_progress_only_counts_while_a_scan_runs() {
        let service = service(
            Arc::new(FakeScanner::default()),
            Arc::new(FakeTracks::default()),
        );

        service.report_native_progress(5, 100, "reading");
        assert_eq!(service.status().total, 0, "nothing is running");

        let entered = Arc::new(Notify::new());
        let release = Arc::new(Notify::new());
        let blocking = Arc::new(ScanService::new(
            Arc::new(BlockingScanner {
                entered: entered.clone(),
                release: release.clone(),
            }),
            Arc::new(FakeTracks::default()),
        ));
        let running = {
            let blocking = blocking.clone();
            tokio::spawn(async move { blocking.scan(&[], ScanMode::Full).await })
        };
        entered.notified().await;

        blocking.report_native_progress(5, 100, "reading");
        let status = blocking.status();
        assert_eq!(status.scanned, 5);
        assert_eq!(status.total, 100);
        assert_eq!(status.phase, "reading");

        release.notify_one();
        running.await.expect("join").expect("scan");
    }

    #[tokio::test]
    async fn roots_and_artwork_pass_through_with_blank_input_rejected() {
        let scanner = Arc::new(FakeScanner::default());
        *scanner.roots.lock().expect("lock") = vec!["content://tree/a".to_owned()];
        *scanner.picked.lock().expect("lock") = Some("content://tree/b".to_owned());
        let service = service(scanner.clone(), Arc::new(FakeTracks::default()));

        assert_eq!(
            service.roots().await.expect("roots"),
            vec!["content://tree/a".to_owned()]
        );
        assert_eq!(
            service.pick_folder().await.expect("picked"),
            Some("content://tree/b".to_owned())
        );
        service
            .remove_root(" content://tree/a ")
            .await
            .expect("released");
        assert_eq!(
            scanner.released.lock().expect("lock").as_slice(),
            &["content://tree/a".to_owned()],
            "the uri reaches the native side trimmed"
        );

        assert_eq!(
            service
                .remove_root("  ")
                .await
                .expect_err("blank uri")
                .code(),
            "INVALID_INPUT"
        );
        assert_eq!(
            service
                .extract_artwork("")
                .await
                .expect_err("blank uri")
                .code(),
            "INVALID_INPUT"
        );
        assert_eq!(
            service
                .extract_artwork("content://media/1")
                .await
                .expect("artwork"),
            Some("file:///cache/content://media/1.jpg".to_owned())
        );
    }
}
