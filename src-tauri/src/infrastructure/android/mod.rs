//! Bridge to `tauri-plugin-player`: the native player, the native scanner and
//! the events both of them raise (CONTRACTS §6, §7).
//!
//! Everything platform specific stops here. The application layer only sees
//! `PlayerPort` / `ScannerPort` and domain types.

pub mod events;
pub mod player_adapter;
pub mod scanner_adapter;

use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager, Runtime};

use crate::application::ArtworkPort;
use crate::error::{CoreError, CoreResult};

pub use events::{emit_library_changed, emit_scan_progress, subscribe, EventSinks};
pub use player_adapter::AndroidPlayerAdapter;
pub use scanner_adapter::AndroidScannerAdapter;

/// What a call fails with when the current target has no native backing
/// (macOS/Linux dev builds).
pub const UNSUPPORTED: &str = "unsupported on this platform";

pub fn unsupported<T>() -> CoreResult<T> {
    Err(CoreError::Player(UNSUPPORTED.to_owned()))
}

/// Sub-directory of the app cache the native side writes extracted artwork to.
const ARTWORK_DIR: &str = "artwork";
const ARTWORK_EXTENSIONS: [&str; 4] = ["jpg", "jpeg", "png", "webp"];
const URI_SCHEMES: [&str; 5] = ["content://", "file://", "asset://", "http://", "https://"];

/// [`ArtworkPort`] over the app cache: a `coverKey` becomes a URI the WebView
/// is allowed to render (CONTRACTS §5, `artwork_uri`).
pub struct AndroidArtworkAdapter<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> AndroidArtworkAdapter<R> {
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

#[async_trait::async_trait]
impl<R: Runtime> ArtworkPort for AndroidArtworkAdapter<R> {
    async fn artwork_uri(&self, cover_key: &str) -> CoreResult<Option<String>> {
        Ok(artwork_asset_uri(&self.app, cover_key))
    }
}

/// Artwork the native player can read directly (`file://` or the URI the
/// scanner already handed us).
pub fn artwork_file_uri<R: Runtime>(app: &AppHandle<R>, cover_key: &str) -> Option<String> {
    if is_uri(cover_key) {
        return Some(cover_key.to_owned());
    }
    artwork_path(app, cover_key).map(|path| file_uri(&path))
}

/// Artwork the WebView can load: the asset protocol URL, built exactly like
/// `convertFileSrc` builds it on the JS side.
pub fn artwork_asset_uri<R: Runtime>(app: &AppHandle<R>, cover_key: &str) -> Option<String> {
    if is_uri(cover_key) {
        return Some(cover_key.to_owned());
    }
    artwork_path(app, cover_key).map(|path| asset_url(&path))
}

/// Cover keys are flat file names inside the artwork cache; anything with a
/// path separator is rejected instead of being resolved.
fn artwork_path<R: Runtime>(app: &AppHandle<R>, cover_key: &str) -> Option<PathBuf> {
    let key = cover_key.trim();
    if key.is_empty() || key.contains('/') || key.contains('\\') || key.contains("..") {
        return None;
    }

    let dir = app.path().app_cache_dir().ok()?.join(ARTWORK_DIR);
    let direct = dir.join(key);
    if direct.is_file() {
        return Some(direct);
    }

    ARTWORK_EXTENSIONS
        .iter()
        .map(|extension| dir.join(format!("{key}.{extension}")))
        .find(|candidate| candidate.is_file())
}

fn is_uri(value: &str) -> bool {
    let lowered = value.trim().to_ascii_lowercase();
    URI_SCHEMES.iter().any(|scheme| lowered.starts_with(scheme))
}

fn file_uri(path: &Path) -> String {
    format!("file://{}", encode(&path.to_string_lossy(), true))
}

fn asset_url(path: &Path) -> String {
    let encoded = encode(&path.to_string_lossy(), false);
    if cfg!(any(target_os = "android", target_os = "windows")) {
        format!("http://asset.localhost/{encoded}")
    } else {
        format!("asset://localhost/{encoded}")
    }
}

/// `encodeURIComponent` semantics; `keep_slash` keeps path separators readable
/// for `file://` URIs.
fn encode(value: &str, keep_slash: bool) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        let character = *byte as char;
        let unreserved = character.is_ascii_alphanumeric()
            || matches!(
                character,
                '-' | '_' | '.' | '!' | '~' | '*' | '\'' | '(' | ')'
            );
        if unreserved || (keep_slash && character == '/') {
            encoded.push(character);
        } else {
            encoded.push('%');
            encoded.push_str(&format!("{byte:02X}"));
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::{asset_url, encode, file_uri, is_uri, unsupported};
    use crate::error::CoreError;
    use std::path::Path;

    #[test]
    fn unsupported_maps_to_a_player_error() {
        let error = unsupported::<()>().expect_err("unsupported is an error");
        assert_eq!(error.code(), "PLAYER");
        assert_eq!(
            error.to_string(),
            "player error: unsupported on this platform"
        );
        assert!(matches!(error, CoreError::Player(_)));
    }

    #[test]
    fn uris_are_passed_through_untouched() {
        assert!(is_uri("content://media/external/audio/albumart/12"));
        assert!(is_uri("FILE:///data/app/cover.jpg"));
        assert!(!is_uri("a1b2c3"));
        assert!(!is_uri(""));
    }

    #[test]
    fn file_uris_keep_separators_and_escape_the_rest() {
        assert_eq!(
            file_uri(Path::new("/data/cache/artwork/a b.jpg")),
            "file:///data/cache/artwork/a%20b.jpg"
        );
    }

    #[test]
    fn asset_urls_escape_the_whole_path() {
        let url = asset_url(Path::new("/data/cache/artwork/cover.jpg"));
        assert!(
            url.ends_with("%2Fdata%2Fcache%2Fartwork%2Fcover.jpg"),
            "{url}"
        );
        assert!(
            url.starts_with("asset://localhost/") || url.starts_with("http://asset.localhost/"),
            "{url}"
        );
    }

    #[test]
    fn non_ascii_is_percent_encoded_per_utf8_byte() {
        assert_eq!(encode("Ä", false), "%C3%84");
        assert_eq!(encode("a-b_c.d~e", false), "a-b_c.d~e");
    }
}
