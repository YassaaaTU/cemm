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
            // Bounded by ICON_BATCH_FETCH_BUDGET on the sidecar side, so this
            // is headroom over a batch that stops itself rather than a cap the
            // batch is free to run up against.
            Self::LibraryCacheIcons => Duration::from_secs(2 * 60),
            Self::GithubUploadUpdate
            | Self::GithubDownloadManifest
            | Self::GithubDownloadConfigFiles => Duration::from_secs(30 * 60),
            // A large modpack legitimately takes hours to install.
            Self::InstallApplyUpdate => Duration::from_secs(2 * 60 * 60),
        }
    }

    /// Whether this method holds the sidecar for long enough that a second
    /// caller must be turned away rather than made to wait.
    ///
    /// The server answers one request at a time, so everything queues behind a
    /// publish or an install. Queueing is fine when the wait is seconds; it is
    /// not fine when the wait is the length of an upload, because the caller
    /// sits on a spinner with no way to know why. These four are the methods
    /// that legitimately run for minutes or hours, and while one of them is in
    /// flight any new request fails immediately with `busy_message`.
    ///
    /// Exhaustive on purpose, like `timeout`: a new method cannot be added
    /// without deciding which side of this line it falls on.
    pub const fn is_exclusive(self) -> bool {
        match self {
            Self::GithubUploadUpdate
            | Self::GithubDownloadManifest
            | Self::GithubDownloadConfigFiles
            | Self::InstallApplyUpdate => true,
            Self::Ping
            | Self::FileRead
            | Self::FileWrite
            | Self::ConfigReadDirectory
            | Self::PathIsBinary
            | Self::PathValidate
            | Self::ManifestParseInstance
            | Self::ManifestCompare
            | Self::ManifestDiff
            | Self::LibraryScan
            | Self::LibraryCacheIcons => false,
        }
    }

    /// What to tell the user is already running. Shown verbatim, so it names
    /// the operation in their terms rather than in wire-method terms.
    pub const fn busy_message(self) -> &'static str {
        match self {
            Self::GithubUploadUpdate => {
                "CEMM is publishing an update. Wait for it to finish, then try again."
            }
            Self::GithubDownloadManifest | Self::GithubDownloadConfigFiles => {
                "CEMM is downloading an update. Wait for it to finish, then try again."
            }
            Self::InstallApplyUpdate => {
                "CEMM is installing an update. Wait for it to finish, then try again."
            }
            _ => "CEMM is busy with another operation. Wait for it to finish, then try again.",
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

    /// The exclusive set is the one that decides whether a second caller waits
    /// or is turned away, so it is worth stating outright rather than leaving
    /// implicit in a match arm.
    #[test]
    fn only_the_long_running_methods_are_exclusive() {
        let exclusive: Vec<&str> = Method::ALL
            .into_iter()
            .filter(|method| method.is_exclusive())
            .map(Method::as_str)
            .collect();

        assert_eq!(
            exclusive,
            vec![
                "github.upload_update",
                "github.download_manifest",
                "github.download_config_files",
                "install.apply_update",
            ]
        );
    }

    /// Every exclusive method's message is shown to a user who tried to do
    /// something else, so none of them may fall through to the generic arm.
    #[test]
    fn every_exclusive_method_names_what_is_running() {
        for method in Method::ALL {
            if !method.is_exclusive() {
                continue;
            }
            assert!(
                !method.busy_message().contains("another operation"),
                "{method} falls through to the generic busy message"
            );
        }
    }

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
