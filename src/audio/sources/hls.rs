use async_trait::async_trait;
use reqwest::Client;
use songbird::input::Input;
use url::Url;

use crate::audio::codec::encode_track;
use crate::audio::sources::AudioSource;
use crate::model::protocol::{LoadResult, Track, TrackInfo};

pub struct HlsSource {
    client: Client,
}

impl HlsSource {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl AudioSource for HlsSource {
    fn name(&self) -> &'static str {
        "hls"
    }

    fn can_handle(&self, identifier: &str) -> bool {
        let trimmed = identifier.trim();
        (trimmed.starts_with("http://") || trimmed.starts_with("https://")) && trimmed.contains(".m3u8")
    }

    async fn resolve(&self, identifier: &str) -> Option<LoadResult> {
        let uri = identifier.trim();
        let parsed_url = Url::parse(uri).ok()?;

        let title = parsed_url
            .path_segments()
            .and_then(|mut s| s.next_back())
            .filter(|name| !name.is_empty())
            .unwrap_or("HLS Stream")
            .to_string();

        let info = TrackInfo {
            identifier: uri.to_string(),
            is_seekable: false,
            author: parsed_url.host_str().unwrap_or("HLS Stream").to_string(),
            length: 0,
            is_stream: true,
            position: 0,
            title,
            uri: Some(uri.to_string()),
            artwork_url: None,
            isrc: None,
            source_name: "hls".to_string(),
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
        let hls = songbird::input::HlsRequest::new(self.client.clone(), identifier.trim().to_string());
        Some(hls.into())
    }
}
