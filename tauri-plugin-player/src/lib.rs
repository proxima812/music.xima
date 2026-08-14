//! `tauri-plugin-player` — the only bridge between the Rust core and the
//! Android audio engine (CONTRACTS §7).
//!
//! Kotlin owns playback and file access; this crate only forwards calls and
//! decodes what comes back. Outside Android it compiles to the stub in
//! [`desktop`], so the core crate still builds and tests on macOS/Linux.

use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};

mod commands;
mod error;
mod models;

#[cfg(not(target_os = "android"))]
mod desktop;
#[cfg(target_os = "android")]
mod mobile;

pub use error::{Error, Result};
pub use models::*;

#[cfg(not(target_os = "android"))]
pub use desktop::Player;
#[cfg(target_os = "android")]
pub use mobile::Player;

/// Access to the native player from anything that can reach the app state.
pub trait PlayerExt<R: Runtime> {
    fn player(&self) -> &Player<R>;
}

impl<R: Runtime, T: Manager<R>> PlayerExt<R> for T {
    fn player(&self) -> &Player<R> {
        self.state::<Player<R>>().inner()
    }
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("player")
        .invoke_handler(tauri::generate_handler![
            commands::get_state,
            commands::set_queue,
            commands::play,
            commands::pause,
            commands::toggle,
            commands::stop,
            commands::next,
            commands::previous,
            commands::seek,
            commands::skip_to,
            commands::set_shuffle,
            commands::set_repeat,
            commands::set_volume,
            commands::set_speed,
            commands::add_next,
            commands::add_to_queue,
            commands::remove_queue_item,
            commands::move_queue_item,
            commands::clear_queue,
            commands::scan_media_store,
            commands::scan_tree,
            commands::pick_folder,
            commands::persisted_roots,
            commands::release_root,
            commands::extract_artwork,
        ])
        .setup(|app, api| {
            #[cfg(target_os = "android")]
            let player = mobile::init(app, api)?;
            #[cfg(not(target_os = "android"))]
            let player = desktop::init(app, api)?;

            app.manage(player);
            Ok(())
        })
        .build()
}
