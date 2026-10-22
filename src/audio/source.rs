use reqwest::Client;
use songbird::input::Input;
use std::sync::Arc;

use crate::audio::sources::hls::HlsSource;
use crate::audio::sources::http::HttpSource;
use crate::audio::sources::local::LocalSource;
use crate::audio::sources::youtube::YoutubeSource;
use crate::audio::sources::AudioSource;
use crate::model::protocol::LoadResult;

#[derive(Clone)]
pub struct SourceManager {
    sources: Vec<Arc<dyn AudioSource>>,
}

impl SourceManager {
    pub fn new() -> Self {
        Self::with_defaults()
    }

    pub fn with_defaults() -> Self {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) RustLink/4.0.0")
            .build()
            .unwrap_or_else(|_| Client::new());

        let sources: Vec<Arc<dyn AudioSource>> = vec![
            Arc::new(YoutubeSource::new(client.clone())),
            Arc::new(HlsSource::new(client.clone())),
            Arc::new(HttpSource::new(client)),
            Arc::new(LocalSource::new()),
        ];

        Self { sources }
    }

    pub fn with_enabled(enabled: &[String]) -> Self {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) RustLink/4.0.0")
            .build()
            .unwrap_or_else(|_| Client::new());

        let mut sources: Vec<Arc<dyn AudioSource>> = Vec::new();

        if enabled.iter().any(|s| s == "youtube") {
            sources.push(Arc::new(YoutubeSource::new(client.clone())));
        }
        if enabled.iter().any(|s| s == "hls") {
            sources.push(Arc::new(HlsSource::new(client.clone())));
        }
        if enabled.iter().any(|s| s == "http") {
            sources.push(Arc::new(HttpSource::new(client)));
        }
        if enabled.iter().any(|s| s == "local") {
            sources.push(Arc::new(LocalSource::new()));
        }

        Self { sources }
    }

    pub fn register_source<S: AudioSource + 'static>(&mut self, source: S) {
        self.sources.push(Arc::new(source));
    }

    pub async fn resolve(&self, identifier: &str) -> LoadResult {
        let trimmed = identifier.trim();

        for source in &self.sources {
            if source.can_handle(trimmed) {
                if let Some(result) = source.resolve(trimmed).await {
                    return result;
                }
            }
        }

        LoadResult::Empty(serde_json::json!({}))
    }

    pub fn create_input(&self, uri_or_path: &str) -> Option<Input> {
        let trimmed = uri_or_path.trim();

        for source in &self.sources {
            if source.can_handle(trimmed) {
                if let Some(input) = source.create_input(trimmed) {
                    return Some(input);
                }
            }
        }

        None
    }
}
