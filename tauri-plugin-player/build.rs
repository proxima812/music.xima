/// Tauri commands exposed by this plugin (`plugin:player|<name>`).
///
/// These are the JS-facing names and the identifiers used by the generated
/// permission set. They stay in sync with docs/CONTRACTS.md §7.
const COMMANDS: &[&str] = &[
    // playback
    "get_state",
    "get_queue_ids",
    "set_queue",
    "play",
    "pause",
    "toggle",
    "stop",
    "next",
    "previous",
    "seek",
    "skip_to",
    "set_shuffle",
    "set_repeat",
    "set_volume",
    "set_speed",
    "set_crossfade",
    // queue
    "add_next",
    "add_to_queue",
    "remove_queue_item",
    "move_queue_item",
    "clear_queue",
    // library / storage
    "scan_media_store",
    "scan_tree",
    "pick_folder",
    "persisted_roots",
    "release_root",
    "extract_artwork",
    "delete_track_file",
    "track_file_exists",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS)
        .android_path("android")
        .build();
}
