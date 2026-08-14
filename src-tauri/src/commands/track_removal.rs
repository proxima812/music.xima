//! Hiding, restoration, and confirmed native file deletion commands.

use tauri::{AppHandle, State};

use crate::application::validated_id;
use crate::domain::{DeleteTrackResult, HiddenTrack};
use crate::error::CoreResult;
use crate::infrastructure::android::emit_library_changed;
use crate::state::AppState;

fn track_id(id: i64) -> CoreResult<i64> {
    validated_id("track", id)
}

#[tauri::command]
pub async fn track_hide(app: AppHandle, state: State<'_, AppState>, id: i64) -> CoreResult<()> {
    state.track_removal.hide(track_id(id)?).await?;
    emit_library_changed(&app, "track-hidden");
    Ok(())
}

#[tauri::command]
pub async fn track_restore(app: AppHandle, state: State<'_, AppState>, id: i64) -> CoreResult<()> {
    state.track_removal.restore(track_id(id)?).await?;
    emit_library_changed(&app, "track-restored");
    Ok(())
}

#[tauri::command]
pub async fn track_hidden(state: State<'_, AppState>) -> CoreResult<Vec<HiddenTrack>> {
    state.track_removal.hidden().await
}

#[tauri::command]
pub async fn track_delete_file(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
) -> CoreResult<DeleteTrackResult> {
    let result = state.track_removal.delete_file(track_id(id)?).await?;
    if result == DeleteTrackResult::Deleted {
        emit_library_changed(&app, "track-deleted");
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::track_id;
    use crate::domain::DeleteTrackResult;
    use crate::error::CoreError;

    #[test]
    fn command_ids_must_be_positive() {
        assert_eq!(track_id(1).expect("positive id"), 1);
        assert_eq!(
            track_id(0).expect_err("zero rejected").code(),
            "INVALID_INPUT"
        );
        assert_eq!(
            track_id(-1).expect_err("negative rejected").code(),
            "INVALID_INPUT"
        );
    }

    #[test]
    fn cancellation_serializes_as_the_stable_wire_value() {
        assert_eq!(
            serde_json::to_value(DeleteTrackResult::Cancelled).expect("serializes"),
            json!("cancelled")
        );
    }

    #[test]
    fn service_errors_keep_their_stable_ipc_codes() {
        assert_eq!(
            serde_json::to_value(CoreError::Player("provider failure".to_owned()))
                .expect("serializes"),
            json!({ "code": "PLAYER", "message": "player error: provider failure" })
        );
    }
}
