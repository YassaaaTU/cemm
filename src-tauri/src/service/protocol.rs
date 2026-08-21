use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const SERVICE_PROTOCOL_VERSION: u32 = 1;

/// Every method the sidecar answers, and the deadline each one gets.
///
/// The wire name, the dispatch arm and the timeout used to be three separate
/// string literals in three files, kept in agreement by hand. They agreed, but
/// nothing made them: a rename that missed the timeout table did not fail to
/// compile and did not fail a test, it silently moved that method onto the
/// fallback deadline. For `install.apply_update` that meant dropping from two
/// hours to five minutes, on the one code path that writes into a live game
/// directory.
///
/// Naming them once, as a type, is what makes the compiler check the agreement.
/// There is deliberately no catch-all arm in `timeout`, so a new variant cannot
/// be added without choosing its deadline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Method {
    Ping,
    FileRead,
    FileWrite,
    ConfigReadDirectory,
    PathIsBinary,
    PathValidate,
    ManifestParseInstance,
    ManifestCompare,
    ManifestDiff,
    GithubUploadUpdate,
    GithubDownloadManifest,
    GithubDownloadConfigFiles,
    InstallApplyUpdate,
    LibraryScan,
    LibraryCacheIcons,
}

impl Method {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ping => "ping",
            Self::FileRead => "file.read",
            Self::FileWrite => "file.write",
            Self::ConfigReadDirectory => "config.read_directory",
            Self::PathIsBinary => "path.is_binary",
            Self::PathValidate => "path.validate",
            Self::ManifestParseInstance => "manifest.parse_instance",
            Self::ManifestCompare => "manifest.compare",
            Self::ManifestDiff => "manifest.diff",
            Self::GithubUploadUpdate => "github.upload_update",
            Self::GithubDownloadManifest => "github.download_manifest",
            Self::GithubDownloadConfigFiles => "github.download_config_files",
            Self::InstallApplyUpdate => "install.apply_update",
            Self::LibraryScan => "library.scan",
            Self::LibraryCacheIcons => "library.cache_icons",
        }
    }

    pub fn from_wire(method: &str) -> Option<Self> {
        Some(match method {
            "ping" => Self::Ping,
            "file.read" => Self::FileRead,
            "file.write" => Self::FileWrite,
            "config.read_directory" => Self::ConfigReadDirectory,
            "path.is_binary" => Self::PathIsBinary,
            "path.validate" => Self::PathValidate,
            "manifest.parse_instance" => Self::ManifestParseInstance,
            "manifest.compare" => Self::ManifestCompare,
            "manifest.diff" => Self::ManifestDiff,
            "github.upload_update" => Self::GithubUploadUpdate,
            "github.download_manifest" => Self::GithubDownloadManifest,
            "github.download_config_files" => Self::GithubDownloadConfigFiles,
            "install.apply_update" => Self::InstallApplyUpdate,
            "library.scan" => Self::LibraryScan,
            "library.cache_icons" => Self::LibraryCacheIcons,
            _ => return None,
        })
    }

    /// How long the host waits before declaring the child unresponsive and
    /// killing it. Exhaustive on purpose -- see the type's documentation.
    pub const fn timeout(self) -> Duration {
        match self {
            Self::Ping => Duration::from_secs(10),
            Self::FileRead
            | Self::FileWrite
            | Self::ConfigReadDirectory
            | Self::PathIsBinary
            | Self::PathValidate
            | Self::ManifestParseInstance
            | Self::ManifestCompare
            | Self::ManifestDiff
            | Self::LibraryScan => Duration::from_secs(120),
            Self::LibraryCacheIcons => Duration::from_secs(10 * 60),
            Self::GithubUploadUpdate
            | Self::GithubDownloadManifest
            | Self::GithubDownloadConfigFiles => Duration::from_secs(30 * 60),
            // A large modpack legitimately takes hours to install.
            Self::InstallApplyUpdate => Duration::from_secs(2 * 60 * 60),
        }
    }

    /// Every method, for tests that assert the wire mapping is a bijection.
    pub const ALL: [Method; 15] = [
        Self::Ping,
        Self::FileRead,
        Self::FileWrite,
        Self::ConfigReadDirectory,
        Self::PathIsBinary,
        Self::PathValidate,
        Self::ManifestParseInstance,
        Self::ManifestCompare,
        Self::ManifestDiff,
        Self::GithubUploadUpdate,
        Self::GithubDownloadManifest,
        Self::GithubDownloadConfigFiles,
        Self::InstallApplyUpdate,
        Self::LibraryScan,
        Self::LibraryCacheIcons,
    ];
}

impl std::fmt::Display for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServiceRequest {
    pub id: u64,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServiceMessage {
    Ready {
        protocol_version: u32,
    },
    Response {
        id: u64,
        result: Value,
    },
    Error {
        id: u64,
        error: String,
    },
    Event {
        id: u64,
        name: String,
        payload: Value,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ServiceEvent {
    pub request_id: u64,
    pub name: String,
    pub payload: Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_round_trips_as_one_json_value() {
        let request = ServiceRequest {
            id: 42,
            method: "ping".to_string(),
            params: serde_json::json!({ "probe": true }),
        };

        let encoded = serde_json::to_string(&request).expect("request should serialize");
        assert!(!encoded.contains('\n'));
        assert_eq!(
            serde_json::from_str::<ServiceRequest>(&encoded).expect("request should deserialize"),
            request
        );
    }

    #[test]
    fn tagged_messages_keep_response_and_event_shapes_distinct() {
        let response = ServiceMessage::Response {
            id: 7,
            result: serde_json::json!({ "ok": true }),
        };
        let event = ServiceMessage::Event {
            id: 7,
            name: "install-progress".to_string(),
            payload: serde_json::json!({ "progress": 25 }),
        };

        let response_json = serde_json::to_value(response).expect("response should serialize");
        let event_json = serde_json::to_value(event).expect("event should serialize");

        assert_eq!(response_json["type"], "response");
        assert_eq!(event_json["type"], "event");
    }
}
