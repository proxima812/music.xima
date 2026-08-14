//! `ScannerPort` implemented on top of `tauri-plugin-player` (CONTRACTS §7.1).
//!
//! MediaStore and SAF live on the Kotlin side; this adapter only forwards the
//! call and re-shapes the answer into domain types. Scans can take seconds, so
//! every native call runs on a blocking task instead of on an async worker.
//!
//! The `cursor` the port passes in is deliberately dropped: CONTRACTS §7.1
//! exposes no cursor parameter, because the Kotlin scanner keeps the paging
//! session itself and continues it on the next call with the same `since`.
//! `next_cursor` still travels back up so the service can tell "more to come"
//! from "same page again" and stop.

use tauri::{AppHandle, Runtime};
use tauri_plugin_player::{
    Player, PlayerExt, Result as PluginResult, ScanBatch as NativeScanBatch,
    ScannedTrack as NativeScannedTrack,
};

use crate::application::scan_service::{ScanBatch, ScannerPort};
use crate::domain::ScannedTrack;
use crate::error::{CoreError, CoreResult};

pub struct AndroidScannerAdapter<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> AndroidScannerAdapter<R> {
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }

    /// Off the async workers: `run_mobile_plugin` blocks until Kotlin answers,
    /// and a full library scan is not a quick answer.
    async fn call<T, F>(&self, call: F) -> CoreResult<T>
    where
        T: Send + 'static,
        F: FnOnce(&Player<R>) -> PluginResult<T> + Send + 'static,
    {
        let app = self.app.clone();
        tauri::async_runtime::spawn_blocking(move || call(app.player()))
            .await
            .map_err(|error| CoreError::Scan(format!("scanner task failed: {error}")))?
            .map_err(CoreError::from)
    }
}

#[async_trait::async_trait]
impl<R: Runtime> ScannerPort for AndroidScannerAdapter<R> {
    async fn scan_media_store(
        &self,
        since: Option<i64>,
        _cursor: Option<&str>,
    ) -> CoreResult<ScanBatch> {
        let batch = self
            .call(move |player| player.scan_media_store(since))
            .await?;
        Ok(scan_batch(batch))
    }

    async fn scan_tree(
        &self,
        tree_uri: &str,
        since: Option<i64>,
        _cursor: Option<&str>,
    ) -> CoreResult<ScanBatch> {
        let tree_uri = tree_uri.to_owned();
        let batch = self
            .call(move |player| player.scan_tree(tree_uri, since))
            .await?;
        Ok(scan_batch(batch))
    }

    /// `None` when the user dismissed the system picker.
    async fn pick_folder(&self) -> CoreResult<Option<String>> {
        self.call(|player| player.pick_folder()).await
    }

    async fn persisted_roots(&self) -> CoreResult<Vec<String>> {
        self.call(|player| player.persisted_roots()).await
    }

    async fn release_root(&self, tree_uri: &str) -> CoreResult<()> {
        let tree_uri = tree_uri.to_owned();
        self.call(move |player| player.release_root(tree_uri)).await
    }

    async fn extract_artwork(&self, uri: &str) -> CoreResult<Option<String>> {
        let uri = uri.to_owned();
        self.call(move |player| player.extract_artwork(uri)).await
    }
}

fn scan_batch(batch: NativeScanBatch) -> ScanBatch {
    ScanBatch {
        tracks: batch.tracks.into_iter().map(scanned_track).collect(),
        complete: batch.complete,
        next_cursor: batch.next_cursor,
    }
}

/// Copied field by field on purpose: the plugin keeps its own DTOs, and a
/// change on either side must fail the build instead of silently dropping data.
fn scanned_track(track: NativeScannedTrack) -> ScannedTrack {
    ScannedTrack {
        uri: track.uri,
        title: track.title,
        artist: track.artist,
        album: track.album,
        album_artist: track.album_artist,
        duration_ms: track.duration_ms,
        track_number: track.track_number,
        disc_number: track.disc_number,
        year: track.year,
        genre: track.genre,
        bitrate: track.bitrate,
        sample_rate: track.sample_rate,
        size: track.size,
        mime_type: track.mime_type,
        folder: track.folder,
        date_added: track.date_added,
        last_modified: track.last_modified,
        cover_key: track.cover_key,
    }
}

#[cfg(test)]
mod tests {
    use super::{scan_batch, scanned_track};
    use tauri_plugin_player::{ScanBatch as NativeScanBatch, ScannedTrack as NativeScannedTrack};

    fn native() -> NativeScannedTrack {
        NativeScannedTrack {
            uri: "content://tree/1".to_owned(),
            title: "Song".to_owned(),
            artist: Some("Artist".to_owned()),
            album: Some("Album".to_owned()),
            album_artist: Some("Album Artist".to_owned()),
            duration_ms: 180_000,
            track_number: Some(3),
            disc_number: Some(1),
            year: Some(2021),
            genre: Some("Rock".to_owned()),
            bitrate: Some(320_000),
            sample_rate: Some(44_100),
            size: 7_200_000,
            mime_type: Some("audio/mpeg".to_owned()),
            folder: Some("Music/Rock".to_owned()),
            date_added: 1_700_000_000_000,
            last_modified: 1_700_000_100_000,
            cover_key: Some("cover-1".to_owned()),
        }
    }

    #[test]
    fn every_field_survives_the_boundary() {
        let track = scanned_track(native());
        assert_eq!(track.uri, "content://tree/1");
        assert_eq!(track.album_artist.as_deref(), Some("Album Artist"));
        assert_eq!(track.mime_type.as_deref(), Some("audio/mpeg"));
        assert_eq!(track.format_label().as_deref(), Some("MP3"));
        assert_eq!(track.sample_rate, Some(44_100));
        assert_eq!(track.last_modified, 1_700_000_100_000);
        assert_eq!(track.cover_key.as_deref(), Some("cover-1"));
    }

    #[test]
    fn batches_keep_their_cursor() {
        let batch = scan_batch(NativeScanBatch {
            tracks: vec![native()],
            complete: false,
            next_cursor: Some("42".to_owned()),
        });
        assert_eq!(batch.tracks.len(), 1);
        assert!(!batch.complete);
        assert_eq!(batch.next_cursor.as_deref(), Some("42"));

        let done = scan_batch(NativeScanBatch::empty());
        assert!(done.complete);
        assert!(done.tracks.is_empty());
        assert!(done.next_cursor.is_none());
    }
}
