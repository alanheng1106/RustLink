use async_trait::async_trait;
use reqwest::Client;
use songbird::input::Input;

use crate::audio::codec::encode_track;
use crate::audio::sources::AudioSource;
use crate::model::protocol::{ExceptionData, LoadResult, Track, TrackInfo};

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

pub struct YoutubeSource {
    client: Client,
}

impl YoutubeSource {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    pub fn is_youtube_url(uri: &str) -> bool {
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
}

#[async_trait]
impl AudioSource for YoutubeSource {
    fn name(&self) -> &'static str {
        "youtube"
    }

    fn can_handle(&self, identifier: &str) -> bool {
        let trimmed = identifier.trim();
        trimmed.starts_with("ytsearch:") || Self::is_youtube_url(trimmed)
    }

    async fn resolve(&self, identifier: &str) -> Option<LoadResult> {
        let trimmed = identifier.trim();

        if trimmed.starts_with("ytsearch:") {
            let clean_query = trimmed.strip_prefix("ytsearch:").unwrap_or(trimmed).trim();
            let search_target = format!("ytsearch10:{clean_query}");

            let output_res = tokio::process::Command::new("yt-dlp")
                .args(["--flat-playlist", "-j", &search_target, "--no-playlist"])
                .output()
                .await;

            return match output_res {
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
                            let video_url = entry
                                .url
                                .unwrap_or_else(|| format!("https://www.youtube.com/watch?v={video_id}"));
                            let title = entry.title.unwrap_or_else(|| "Unknown Title".to_string());
                            let author = entry.uploader.or(entry.channel).unwrap_or_else(|| "YouTube".to_string());
                            let length = entry.duration.map(|d| (d * 1000.0) as u64).unwrap_or(0);
                            let artwork_url =
                                entry.thumbnails.and_then(|t| t.into_iter().next_back().and_then(|x| x.url));

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
                        Some(LoadResult::Empty(serde_json::json!({})))
                    } else {
                        Some(LoadResult::Search(tracks))
                    }
                }
                Ok(output) => {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    tracing::warn!("yt-dlp search non-zero: {stderr}");
                    Some(LoadResult::Error(ExceptionData {
                        message: format!("YouTube search failed: {stderr}"),
                        severity: "common".to_string(),
                        cause: stderr.to_string(),
                    }))
                }
                Err(e) => {
                    tracing::warn!("Failed to spawn yt-dlp: {e}");
                    Some(LoadResult::Error(ExceptionData {
                        message: format!("Failed to spawn yt-dlp: {e}"),
                        severity: "common".to_string(),
                        cause: e.to_string(),
                    }))
                }
            };
        }

        if Self::is_youtube_url(trimmed) {
            let output_res = tokio::process::Command::new("yt-dlp")
                .args(["--flat-playlist", "-j", trimmed, "--no-playlist"])
                .output()
                .await;

            return match output_res {
                Ok(output) if output.status.success() => {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    if let Some(line) = stdout.lines().find(|l| !l.trim().is_empty()) {
                        if let Ok(entry) = serde_json::from_str::<YtDlpFlatEntry>(line.trim()) {
                            let video_id = entry.id.unwrap_or_else(|| trimmed.to_string());
                            let video_url = entry.url.unwrap_or_else(|| trimmed.to_string());
                            let title = entry.title.unwrap_or_else(|| "YouTube Video".to_string());
                            let author = entry.uploader.or(entry.channel).unwrap_or_else(|| "YouTube".to_string());
                            let length = entry.duration.map(|d| (d * 1000.0) as u64).unwrap_or(0);
                            let artwork_url =
                                entry.thumbnails.and_then(|t| t.into_iter().next_back().and_then(|x| x.url));

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
                            return Some(LoadResult::Track(Track {
                                encoded,
                                info,
                                plugin_info: serde_json::json!({}),
                                user_data: serde_json::json!({}),
                            }));
                        }
                    }
                    Some(LoadResult::Empty(serde_json::json!({})))
                }
                Ok(output) => {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    tracing::warn!("yt-dlp url resolution non-zero: {stderr}");
                    Some(LoadResult::Error(ExceptionData {
                        message: format!("Failed to extract YouTube video: {stderr}"),
                        severity: "common".to_string(),
                        cause: stderr.to_string(),
                    }))
                }
                Err(e) => {
                    tracing::warn!("Failed to spawn yt-dlp: {e}");
                    Some(LoadResult::Error(ExceptionData {
                        message: format!("Failed to spawn yt-dlp: {e}"),
                        severity: "common".to_string(),
                        cause: e.to_string(),
                    }))
                }
            };
        }

        None
    }

    fn create_input(&self, identifier: &str) -> Option<Input> {
        let trimmed = identifier.trim();
        let ytdl = songbird::input::YoutubeDl::new(self.client.clone(), trimmed.to_string())
            .user_args(ytdl_args());
        Some(ytdl.into())
    }
}
