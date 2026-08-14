//! music.xima core: composition root and the Tauri entry point.
//!
//! Wiring order is the dependency order of the layers — pool, repositories
//! (`infrastructure::sqlite`), native adapters (`infrastructure::android`),
//! services (`application`), state, commands (CONTRACTS §5).

pub mod application;
pub mod commands;
pub mod domain;
pub mod error;
pub mod infrastructure;
pub mod state;

use std::sync::Arc;

use tauri::{App, Manager, Runtime};

use crate::application::{
    HistoryService, LibraryService, PlayerService, PlaylistService, ProgressCallback, ScanService,
    SearchService, StatisticsService,
};
use crate::error::CoreResult;
use crate::infrastructure::android::{
    emit_scan_progress, subscribe, AndroidArtworkAdapter, AndroidPlayerAdapter,
    AndroidScannerAdapter, EventSinks,
};
use crate::infrastructure::sqlite::{
    self, SqliteAlbumRepository, SqliteArtistRepository, SqliteFavoriteRepository,
    SqliteHistoryRepository, SqlitePlaylistRepository, SqliteSearchRepository,
    SqliteSmartPlaylistRepository, SqliteStatisticsRepository, SqliteTaxonomyRepository,
    SqliteTrackRepository, DB_FILE_NAME,
};
use crate::state::AppState;

/// On Android the app is started by the JVM, not by `main`. The macro exports
/// the JNI entry point the generated `MainActivity` loads from the shared
/// library; without it the `.so` builds fine and then fails validation.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_player::init());

    // Mobile targets log through the platform; the desktop dev build needs a
    // sink of its own.
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    let builder = builder.plugin(tauri_plugin_log::Builder::new().build());

    builder
        .setup(|app| {
            setup(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::library::library_stats,
            commands::library::library_tracks,
            commands::library::library_track,
            commands::library::library_tracks_by_ids,
            commands::library::library_recently_added,
            commands::library::library_albums,
            commands::library::library_album,
            commands::library::library_album_tracks,
            commands::library::library_artists,
            commands::library::library_artist,
            commands::library::library_artist_albums,
            commands::library::library_artist_tracks,
            commands::library::library_genres,
            commands::library::library_folders,
            commands::library::library_scan,
            commands::library::library_scan_status,
            commands::library::library_pick_folder,
            commands::library::library_roots,
            commands::library::library_remove_root,
            commands::library::artwork_uri,
            commands::search::search_all,
            commands::search::search_tracks,
            commands::playlist::playlist_list,
            commands::playlist::playlist_get,
            commands::playlist::playlist_create,
            commands::playlist::playlist_rename,
            commands::playlist::playlist_delete,
            commands::playlist::playlist_tracks,
            commands::playlist::playlist_add_tracks,
            commands::playlist::playlist_remove_at,
            commands::playlist::playlist_reorder,
            commands::playlist::smart_playlist_list,
            commands::playlist::smart_playlist_get,
            commands::playlist::smart_playlist_create,
            commands::playlist::smart_playlist_update,
            commands::playlist::smart_playlist_delete,
            commands::playlist::smart_playlist_resolve,
            commands::playlist::smart_playlist_preview,
            commands::favorites::favorite_toggle,
            commands::favorites::favorite_list,
            commands::history::history_record,
            commands::history::history_recent,
            commands::statistics::stats_top_tracks,
            commands::statistics::stats_never_played,
            commands::statistics::stats_forgotten,
            commands::player::player_state,
            commands::player::player_set_queue,
            commands::player::player_queue,
            commands::player::player_play,
            commands::player::player_pause,
            commands::player::player_toggle,
            commands::player::player_stop,
            commands::player::player_next,
            commands::player::player_previous,
            commands::player::player_seek,
            commands::player::player_skip_to,
            commands::player::player_set_shuffle,
            commands::player::player_set_repeat,
            commands::player::player_set_volume,
            commands::player::player_set_speed,
            commands::player::player_add_next,
            commands::player::player_add_to_queue,
            commands::player::player_remove_queue_item,
            commands::player::player_move_queue_item,
            commands::player::player_clear_queue,
        ])
        .run(tauri::generate_context!())
        .expect("music.xima failed to start");
}

/// Opens the database, wires every layer together and manages the result.
/// Runs on the main thread, so the async parts go through `block_on`.
fn setup<R: Runtime>(app: &mut App<R>) -> CoreResult<()> {
    let handle = app.handle().clone();

    let db_path = app.path().app_data_dir()?.join(DB_FILE_NAME);
    let db = tauri::async_runtime::block_on(sqlite::connect(&db_path))?;
    log::info!("library database at {}", db_path.display());

    let scan_progress: ProgressCallback = {
        let handle = handle.clone();
        Box::new(move |status| emit_scan_progress(&handle, status))
    };

    let tracks = Arc::new(SqliteTrackRepository::new(db.clone()));
    let queue = Arc::new(AndroidPlayerAdapter::new(handle.clone()));

    let library = Arc::new(LibraryService::new(
        tracks.clone(),
        Arc::new(SqliteAlbumRepository::new(db.clone())),
        Arc::new(SqliteArtistRepository::new(db.clone())),
        Arc::new(SqliteTaxonomyRepository::new(db.clone())),
        Arc::new(SqliteFavoriteRepository::new(db.clone())),
        Arc::new(AndroidArtworkAdapter::new(handle.clone())),
    ));
    let search = Arc::new(SearchService::new(Arc::new(SqliteSearchRepository::new(
        db.clone(),
    ))));
    let playlists = Arc::new(PlaylistService::new(
        Arc::new(SqlitePlaylistRepository::new(db.clone())),
        Arc::new(SqliteSmartPlaylistRepository::new(db.clone())),
    ));
    let history = Arc::new(HistoryService::new(Arc::new(SqliteHistoryRepository::new(
        db.clone(),
    ))));
    let statistics = Arc::new(StatisticsService::new(Arc::new(
        SqliteStatisticsRepository::new(db.clone()),
    )));
    let player = Arc::new(PlayerService::new(tracks.clone(), queue.clone()));
    let scan = Arc::new(
        ScanService::new(Arc::new(AndroidScannerAdapter::new(handle.clone())), tracks)
            .with_progress(scan_progress),
    );

    subscribe(
        &handle,
        EventSinks {
            queue,
            player: player.clone(),
            history: history.clone(),
            scan: scan.clone(),
        },
    )?;

    app.manage(AppState::new(
        library, search, playlists, history, statistics, player, scan,
    ));
    Ok(())
}
