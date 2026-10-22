use async_trait::async_trait;
use songbird::input::Input;
use std::path::{Path, PathBuf};

use crate::audio::codec::encode_track;
use crate::audio::sources::AudioSource;
use crate::model::protocol::{LoadResult, Track, TrackInfo};

pub struct LocalSource;

impl LocalSource {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl AudioSource for LocalSource {
    fn name(&self) -> &'static str {
        "local"
    }

    fn can_handle(&self, identifier: &str) -> bool {
        let trimmed = identifier.trim();
        trimmed.starts_with("file://") || Path::new(trimmed).is_file()
    }

    async fn resolve(&self, identifier: &str) -> Option<LoadResult> {
        let trimmed = identifier.trim();
        let clean_path = trimmed.trim_start_matches("file://");
        let path = Path::new(clean_path);

        if !path.exists() {
            return None;
        }

        let title = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Local Audio")
            .to_string();

        let info = TrackInfo {
            identifier: path.to_string_lossy().to_string(),
            is_seekable: true,
            author: "Local File".to_string(),
            length: 0,
            is_stream: false,
            position: 0,
            title,
            uri: Some(path.to_string_lossy().to_string()),
            artwork_url: None,
            isrc: None,
            source_name: "local".to_string(),
        };

        let encoded = encode_track(&info);
        Some(LoadResult::Track(Track {
            encoded,
            info,
            plugin_info: serde_json::json!({}),
            user_data: serde_json::json!({}),
        }))
    }

    fn create_input(&self, identifier: &str) -> Option<Input> {
        let clean_path = identifier.trim().trim_start_matches("file://");
        let path = PathBuf::from(clean_path);
        if path.exists() {
            let file = songbird::input::File::new(path);
            Some(file.into())
        } else {
            None
        }
    }
}
