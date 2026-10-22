pub mod hls;
pub mod http;
pub mod local;
pub mod youtube;

use async_trait::async_trait;
use songbird::input::Input;
use crate::model::protocol::LoadResult;

#[async_trait]
pub trait AudioSource: Send + Sync {
    /// Identifier name of the source (e.g. "youtube", "http", "local", "hls")
    fn name(&self) -> &'static str;

    /// Checks if this source can handle the given identifier
    fn can_handle(&self, identifier: &str) -> bool;

    /// Resolves metadata for the identifier (/v4/loadtracks)
    async fn resolve(&self, identifier: &str) -> Option<LoadResult>;

    /// Creates an audio stream input for playback (/v4/sessions/.../players/...)
    fn create_input(&self, identifier: &str) -> Option<Input>;
}
