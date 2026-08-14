//! Plugin errors. The core crate maps these into `CoreError::Player`
//! (CONTRACTS §2) by `to_string()`, so the messages are part of the contract.

use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The Kotlin plugin rejected the call, or the JNI bridge failed.
    #[cfg(target_os = "android")]
    #[error(transparent)]
    PluginInvoke(#[from] tauri::plugin::mobile::PluginInvokeError),

    /// The native player exists only on Android; the desktop stub answers this
    /// to every playback command.
    #[error("player is not supported on this platform")]
    Unsupported,

    /// Stable storage-specific error used by removal recovery and UI handling.
    #[error("UNSUPPORTED_DELETE: native track deletion is not supported")]
    UnsupportedDelete,

    /// The native side answered, but with something the plugin cannot use.
    #[error("native player error: {0}")]
    Native(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub fn native(message: impl Into<String>) -> Self {
        Self::Native(message.into())
    }
}

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::Error;

    #[test]
    fn errors_serialize_as_their_message() {
        assert_eq!(
            serde_json::to_string(&Error::Unsupported).expect("serializes"),
            "\"player is not supported on this platform\""
        );
        assert_eq!(
            serde_json::to_string(&Error::native("no media session")).expect("serializes"),
            "\"native player error: no media session\""
        );
        assert_eq!(
            Error::UnsupportedDelete.to_string(),
            "UNSUPPORTED_DELETE: native track deletion is not supported"
        );
    }
}
