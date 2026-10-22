use reqwest::Client;
use std::path::Path;
use url::Url;
use crate::audio::codec::encode_track;
use crate::model::protocol::{ExceptionData, LoadResult, Track, TrackInfo};

fn is_youtube_url(uri: &str) -> bool {
    let lower = uri.to_lowercase();
    lower.starts_with("https://www.youtube.com/")
        || lower.starts_with("http://www.youtube.com/")
        || lower.starts_with("https://youtube.com/")
        || lower.starts_with("http://youtube.com/")
        || lower.starts_with("https://youtu.be/")
        || lower.starts_with("http://youtu.be/")
        || lower.starts_with("https://music.youtube.com/")
        || lower.starts_with("http://music.youtube.com/")
}

fn ytdl_args() -> Vec<String> {
    vec![
        "--js-runtimes".to_string(),
        "quickjs".to_string(),
        "--remote-components".to_string(),
        "ejs:github".to_string(),
    ]
}

#[derive(serde::Deserialize)]
struct YtDlpThumbnail {
    url: Option<String>,
}

#[derive(serde::Deserialize)]
struct YtDlpFlatEntry {
    id: Option<String>,
    url: Option<String>,
    title: Option<String>,
    duration: Option<f64>,
    uploader: Option<String>,
    channel: Option<String>,
    thumbnails: Option<Vec<YtDlpThumbnail>>,
}

#[derive(Clone)]
pub struct SourceManager {
    client: Client,
}

impl SourceManager {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) RustLink/4.0.0")
                .build()
                .unwrap_or_else(|_| Client::new()),
        }
    }

    pub async fn resolve(&self, identifier: &str) -> LoadResult {
        let trimmed = identifier.trim();

        // 1. Check YouTube search query
        if trimmed.starts_with("ytsearch:") {
            return self.resolve_yt_search(trimmed).await;
        }

        // 2. Check YouTube direct URL
        if is_youtube_url(trimmed) {
            return self.resolve_youtube_url(trimmed).await;
        }

        // 3. Check general HTTP/HTTPS URL
        if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
            return self.resolve_http(trimmed).await;
        }

        // 4. Check local file URL or path
        if trimmed.starts_with("file://") || Path::new(trimmed).is_file() {
            return self.resolve_local(trimmed);
        }

        // 5. Fallback: Empty result
        LoadResult::Empty(serde_json::json!({}))
    }

    async fn resolve_yt_search(&self, query: &str) -> LoadResult {
        let clean_query = query.strip_prefix("ytsearch:").unwrap_or(query).trim();
        let search_target = format!("ytsearch10:{clean_query}");

        let output_res = tokio::process::Command::new("yt-dlp")
            .args(["--flat-playlist", "-j", &search_target, "--no-playlist"])
            .output()
            .await;

        match output_res {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let mut tracks = Vec::new();

                for line in stdout.lines() {
                    let trimmed_line = line.trim();
                    if trimmed_line.is_empty() {
                        continue;
                    }
                    if let Ok(entry) = serde_json::from_str::<YtDlpFlatEntry>(trimmed_line) {
                        let video_id = entry.id.unwrap_or_default();
                        let video_url = entry.url.unwrap_or_else(|| format!("https://www.youtube.com/watch?v={video_id}"));
                        let title = entry.title.unwrap_or_else(|| "Unknown Title".to_string());
                        let author = entry.uploader.or(entry.channel).unwrap_or_else(|| "YouTube".to_string());
                        let length = entry.duration.map(|d| (d * 1000.0) as u64).unwrap_or(0);
                        let artwork_url = entry.thumbnails.and_then(|t| t.into_iter().next_back().and_then(|x| x.url));

                        let info = TrackInfo {
                            identifier: video_id,
                            is_seekable: true,
                            author,
                            length,
                            is_stream: false,
                            position: 0,
                            title,
                            uri: Some(video_url),
                            artwork_url,
                            isrc: None,
                            source_name: "youtube".to_string(),
                        };

                        let encoded = encode_track(&info);
                        tracks.push(Track {
                            encoded,
                            info,
                            plugin_info: serde_json::json!({}),
                            user_data: serde_json::json!({}),
                        });
                    }
                }

                if tracks.is_empty() {
                    LoadResult::Empty(serde_json::json!({}))
                } else {
                    LoadResult::Search(tracks)
                }
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                tracing::warn!("yt-dlp search non-zero: {stderr}");
                LoadResult::Error(ExceptionData {
                    message: format!("YouTube search failed: {stderr}"),
                    severity: "common".to_string(),
                    cause: stderr.to_string(),
                })
            }
            Err(e) => {
                tracing::warn!("Failed to spawn yt-dlp: {e}");
                LoadResult::Error(ExceptionData {
                    message: format!("Failed to spawn yt-dlp: {e}"),
                    severity: "common".to_string(),
                    cause: e.to_string(),
                })
            }
        }
    }

    async fn resolve_youtube_url(&self, url: &str) -> LoadResult {
        let output_res = tokio::process::Command::new("yt-dlp")
            .args(["--flat-playlist", "-j", url, "--no-playlist"])
            .output()
            .await;

        match output_res {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if let Some(line) = stdout.lines().find(|l| !l.trim().is_empty()) {
                    if let Ok(entry) = serde_json::from_str::<YtDlpFlatEntry>(line.trim()) {
                        let video_id = entry.id.unwrap_or_else(|| url.to_string());
                        let video_url = entry.url.unwrap_or_else(|| url.to_string());
                        let title = entry.title.unwrap_or_else(|| "YouTube Video".to_string());
                        let author = entry.uploader.or(entry.channel).unwrap_or_else(|| "YouTube".to_string());
                        let length = entry.duration.map(|d| (d * 1000.0) as u64).unwrap_or(0);
                        let artwork_url = entry.thumbnails.and_then(|t| t.into_iter().next_back().and_then(|x| x.url));

                        let info = TrackInfo {
                            identifier: video_id,
                            is_seekable: true,
                            author,
                            length,
                            is_stream: false,
                            position: 0,
                            title,
                            uri: Some(video_url),
                            artwork_url,
                            isrc: None,
                            source_name: "youtube".to_string(),
                        };

                        let encoded = encode_track(&info);
                        return LoadResult::Track(Track {
                            encoded,
                            info,
                            plugin_info: serde_json::json!({}),
                            user_data: serde_json::json!({}),
                        });
                    }
                }
                LoadResult::Empty(serde_json::json!({}))
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                tracing::warn!("yt-dlp url resolution non-zero: {stderr}");
                LoadResult::Error(ExceptionData {
                    message: format!("Failed to extract YouTube video: {stderr}"),
                    severity: "common".to_string(),
                    cause: stderr.to_string(),
                })
            }
            Err(e) => {
                tracing::warn!("Failed to spawn yt-dlp: {e}");
                LoadResult::Error(ExceptionData {
                    message: format!("Failed to spawn yt-dlp: {e}"),
                    severity: "common".to_string(),
                    cause: e.to_string(),
                })
            }
        }
    }

    async fn resolve_http(&self, uri: &str) -> LoadResult {
        let parsed_url = match Url::parse(uri) {
            Ok(u) => u,
            Err(_) => return LoadResult::Empty(serde_json::json!({})),
        };

        // Derive title from filename in path
        let title = parsed_url
            .path_segments()
            .and_then(|mut s| s.next_back())
            .filter(|name| !name.is_empty())
            .unwrap_or("Online Stream")
            .to_string();

        let is_stream = uri.contains(".m3u8") || uri.contains("stream");
        let source_name = if uri.contains(".m3u8") {
            "hls".to_string()
        } else {
            "http".to_string()
        };

        let info = TrackInfo {
            identifier: uri.to_string(),
            is_seekable: !is_stream,
            author: parsed_url.host_str().unwrap_or("HTTP Source").to_string(),
            length: if is_stream { 0 } else { 180_000 }, // Fallback estimated length
            is_stream,
            position: 0,
            title,
            uri: Some(uri.to_string()),
            artwork_url: None,
            isrc: None,
            source_name,
        };

        let encoded = encode_track(&info);
        LoadResult::Track(Track {
            encoded,
            info,
            plugin_info: serde_json::json!({}),
            user_data: serde_json::json!({}),
        })
    }

    fn resolve_local(&self, uri: &str) -> LoadResult {
        let clean_path = uri.trim_start_matches("file://");
        let path = Path::new(clean_path);

        if !path.exists() {
            return LoadResult::Empty(serde_json::json!({}));
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
        LoadResult::Track(Track {
            encoded,
            info,
            plugin_info: serde_json::json!({}),
            user_data: serde_json::json!({}),
        })
    }

    pub fn create_input(&self, uri_or_path: &str) -> Option<songbird::input::Input> {
        let trimmed = uri_or_path.trim();

        // 1. YouTube or ytsearch
        if is_youtube_url(trimmed) || trimmed.starts_with("ytsearch:") {
            let ytdl = songbird::input::YoutubeDl::new(self.client.clone(), trimmed.to_string())
                .user_args(ytdl_args());
            return Some(ytdl.into());
        }

        // 2. HTTP / HLS Stream
        if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
            if trimmed.contains(".m3u8") {
                let hls = songbird::input::HlsRequest::new(self.client.clone(), trimmed.to_string());
                return Some(hls.into());
            } else {
                let req = songbird::input::HttpRequest::new(self.client.clone(), trimmed.to_string());
                return Some(req.into());
            }
        }

        // 3. Local File
        let clean_path = trimmed.trim_start_matches("file://");
        let path = std::path::PathBuf::from(clean_path);
        if path.exists() {
            let file = songbird::input::File::new(path);
            return Some(file.into());
        }

        None
    }
}
