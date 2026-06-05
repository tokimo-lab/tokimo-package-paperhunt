use crate::paper::Paper;
use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait PaperProvider: Send + Sync {
    fn name(&self) -> &str;
    async fn search(
        &self,
        query: &str,
        limit: usize,
        since: Option<&str>,
        until: Option<&str>,
    ) -> Result<Vec<Paper>>;
}
