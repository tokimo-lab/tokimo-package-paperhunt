use anyhow::Result;

use crate::paper::{DownloadEvent, Paper};

/// Stream download events for a paper PDF via an mpsc channel.
pub async fn download_paper_stream(
    paper: &Paper,
    tx: tokio::sync::mpsc::Sender<Result<DownloadEvent>>,
) {
    macro_rules! send {
        ($val:expr) => {
            if tx.send($val).await.is_err() {
                return;
            }
        };
    }

    let url = match paper.pdf_url.as_deref() {
        Some(u) => u,
        None => {
            send!(Err(anyhow::anyhow!("No PDF URL")));
            return;
        }
    };

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .unwrap();

    let resp = match client.get(url).send().await {
        Ok(r) if r.status().is_success() => r,
        Ok(r) => {
            send!(Err(anyhow::anyhow!("HTTP {}", r.status())));
            return;
        }
        Err(e) => {
            send!(Err(e.into()));
            return;
        }
    };

    let total_bytes = resp.content_length();
    let filename = paper.filename();

    send!(Ok(DownloadEvent::FileInfo {
        title: paper.title.clone(),
        authors: paper.authors.clone(),
        filename: filename.clone(),
        total_bytes,
    }));

    let mut downloaded = 0u64;
    let mut resp = resp;
    loop {
        match resp.chunk().await {
            Ok(Some(chunk)) => {
                downloaded += chunk.len() as u64;
                send!(Ok(DownloadEvent::Data {
                    bytes: chunk.to_vec(),
                    downloaded,
                }));
            }
            Ok(None) => break,
            Err(e) => {
                send!(Err(e.into()));
                return;
            }
        }
    }

    send!(Ok(DownloadEvent::Done {
        filename,
        total_bytes: downloaded,
    }));
}

/// Build a Paper from an arXiv ID or DOI for direct download.
pub fn paper_from_id(id: &str) -> Paper {
    if looks_like_arxiv(id) {
        let clean_id = id.strip_prefix("arXiv:").unwrap_or(id);
        Paper {
            title: clean_id.to_string(),
            authors: vec![],
            abstract_text: String::new(),
            published_date: None,
            doi: None,
            arxiv_id: Some(clean_id.to_string()),
            pdf_url: Some(format!("https://arxiv.org/pdf/{}", clean_id)),
            source: "arxiv".to_string(),
            citation_count: None,
        }
    } else {
        Paper {
            title: id.to_string(),
            authors: vec![],
            abstract_text: String::new(),
            published_date: None,
            doi: Some(id.to_string()),
            arxiv_id: None,
            pdf_url: Some(format!("https://doi.org/{}", id)),
            source: "doi".to_string(),
            citation_count: None,
        }
    }
}

fn looks_like_arxiv(id: &str) -> bool {
    let id = id.strip_prefix("arXiv:").unwrap_or(id);
    let parts: Vec<&str> = id.splitn(2, '.').collect();
    if parts.len() != 2 {
        return false;
    }
    parts[0].len() == 4 && parts[0].chars().all(|c| c.is_ascii_digit())
}
