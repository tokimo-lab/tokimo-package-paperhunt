pub mod paper;
pub mod provider;
pub mod arxiv;
pub mod semantic_scholar;
pub mod openalex;
pub mod download;

pub use paper::{Paper, DownloadEvent};
pub use provider::PaperProvider;
pub use download::paper_from_id;

use futures::Stream;
use tokio_stream::wrappers::ReceiverStream;

/// Get all providers.
pub fn get_providers() -> Vec<Box<dyn PaperProvider>> {
    vec![
        Box::new(arxiv::ArxivProvider::new()),
        Box::new(semantic_scholar::SemanticScholarProvider::new()),
        Box::new(openalex::OpenAlexProvider::new()),
    ]
}

/// Stream search results from all providers concurrently.
pub fn search_stream(
    query: impl Into<String>,
    limit: usize,
    since: Option<String>,
    until: Option<String>,
) -> impl Stream<Item = Paper> + Send + 'static {
    let query = query.into();
    let (tx, rx) = tokio::sync::mpsc::channel::<Paper>(256);

    for provider in get_providers() {
        let tx = tx.clone();
        let q = query.clone();
        let s = since.clone();
        let u = until.clone();
        tokio::spawn(async move {
            match tokio::time::timeout(
                std::time::Duration::from_secs(30),
                provider.search(&q, limit, s.as_deref(), u.as_deref()),
            )
            .await
            {
                Ok(Ok(papers)) => {
                    for p in papers {
                        if tx.send(p).await.is_err() {
                            break;
                        }
                    }
                }
                _ => {}
            }
        });
    }
    drop(tx);
    ReceiverStream::new(rx)
}

/// Stream download events for a paper.
pub fn download_paper_stream(
    paper: Paper,
) -> impl Stream<Item = anyhow::Result<DownloadEvent>> + Send + 'static {
    let (tx, rx) = tokio::sync::mpsc::channel(64);
    tokio::spawn(async move {
        download::download_paper_stream(&paper, tx).await;
    });
    ReceiverStream::new(rx)
}
