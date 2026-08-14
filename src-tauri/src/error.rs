//! Core error type and the shape it takes on the IPC boundary (CONTRACTS §2).

use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("not found: {0}")]
    NotFound(String),
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
    #[error("player error: {0}")]
    Player(String),
    #[error("scan error: {0}")]
    Scan(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("internal: {0}")]
    Internal(String),
}

pub type CoreResult<T> = Result<T, CoreError>;

/// Serializable form handed to the frontend. `code` is stable and switchable,
/// `message` is for logs and developer-facing surfaces.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IpcError {
    pub code: &'static str,
    pub message: String,
}

impl CoreError {
    /// One of `NOT_FOUND | INVALID_INPUT | DATABASE | PLAYER | SCAN | IO | INTERNAL`.
    /// Migration failures are database failures as far as the frontend cares.
    pub fn code(&self) -> &'static str {
        match self {
            Self::NotFound(_) => "NOT_FOUND",
            Self::InvalidInput(_) => "INVALID_INPUT",
            Self::Database(_) | Self::Migration(_) => "DATABASE",
            Self::Player(_) => "PLAYER",
            Self::Scan(_) => "SCAN",
            Self::Io(_) => "IO",
            Self::Internal(_) => "INTERNAL",
        }
    }

    pub fn to_ipc(&self) -> IpcError {
        IpcError {
            code: self.code(),
            message: self.to_string(),
        }
    }

    /// `CoreError::not_found("track", 12)` -> "not found: track 12".
    pub fn not_found(entity: &str, id: i64) -> Self {
        Self::NotFound(format!("{entity} {id}"))
    }

    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::InvalidInput(message.into())
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal(message.into())
    }

    pub fn is_not_found(&self) -> bool {
        matches!(self, Self::NotFound(_))
    }
}

impl Serialize for CoreError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.to_ipc().serialize(serializer)
    }
}

impl From<tauri_plugin_player::Error> for CoreError {
    fn from(error: tauri_plugin_player::Error) -> Self {
        Self::Player(error.to_string())
    }
}

/// Smart-playlist rules travel through `smart_playlists.rules_json`, so JSON
/// failures are internal invariants, not user input.
impl From<serde_json::Error> for CoreError {
    fn from(error: serde_json::Error) -> Self {
        Self::Internal(error.to_string())
    }
}

impl From<tauri::Error> for CoreError {
    fn from(error: tauri::Error) -> Self {
        Self::Internal(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{CoreError, IpcError};
    use serde_json::json;

    #[test]
    fn codes_cover_every_variant() {
        assert_eq!(
            CoreError::NotFound("track 1".to_owned()).code(),
            "NOT_FOUND"
        );
        assert_eq!(
            CoreError::InvalidInput("bad".to_owned()).code(),
            "INVALID_INPUT"
        );
        assert_eq!(
            CoreError::Database(sqlx::Error::RowNotFound).code(),
            "DATABASE"
        );
        assert_eq!(CoreError::Player("boom".to_owned()).code(), "PLAYER");
        assert_eq!(CoreError::Scan("boom".to_owned()).code(), "SCAN");
        assert_eq!(CoreError::Io(std::io::Error::other("boom")).code(), "IO");
        assert_eq!(CoreError::internal("boom").code(), "INTERNAL");
    }

    #[test]
    fn serializes_as_ipc_error() {
        let value =
            serde_json::to_value(CoreError::not_found("track", 12)).expect("error serializes");
        assert_eq!(
            value,
            json!({ "code": "NOT_FOUND", "message": "not found: track 12" })
        );
    }

    #[test]
    fn message_matches_display() {
        let error = CoreError::invalid_input("limit must be positive");
        let ipc: IpcError = error.to_ipc();
        assert_eq!(ipc.message, error.to_string());
        assert_eq!(ipc.message, "invalid input: limit must be positive");
        assert_eq!(ipc.code, "INVALID_INPUT");
    }

    #[test]
    fn database_errors_convert_via_question_mark() {
        fn failing() -> Result<(), CoreError> {
            let raw: Result<(), sqlx::Error> = Err(sqlx::Error::RowNotFound);
            raw?;
            Ok(())
        }
        let error = failing().expect_err("propagates as CoreError");
        assert_eq!(error.code(), "DATABASE");
        assert!(!error.is_not_found());
    }

    #[test]
    fn json_errors_are_internal() {
        let parse_error = serde_json::from_str::<Vec<i64>>("{oops").expect_err("invalid json");
        let error = CoreError::from(parse_error);
        assert_eq!(error.code(), "INTERNAL");
    }
}
