use std::fs;
use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result};
use colored::Colorize;

use crate::paper::{DownloadEvent, Paper};

/// Download the PDF for a single paper into `output_dir`.
/// Returns the path of the downloaded file on success.
pub async fn download_paper(paper: &Paper, output_dir: &Path) -> Result<String> {
    let url = paper
        .pdf_url
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("No PDF URL available for: {}", paper.title))?;

    fs::create_dir_all(output_dir).context("Failed to create output directory")?;

    let filename = paper.filename();
    let dest = output_dir.join(&filename);

    println!(
        "  {} {} ...",
        "Downloading".cyan(),
        filename.dimmed()
    );

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .context("Failed to build HTTP client")?;

    let resp = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("Failed to download from {}", url))?;

    if !resp.status().is_success() {
        anyhow::bail!("HTTP {} when downloading {}", resp.status(), url);
    }

    let bytes = resp.bytes().await.context("Failed to read response body")?;

    let mut file = fs::File::create(&dest)
        .with_context(|| format!("Failed to create file {}", dest.display()))?;
    file.write_all(&bytes)
        .context("Failed to write PDF to disk")?;

    println!(
        "  {} {} ({})",
        "Saved".green(),
        filename,
        human_size(bytes.len())
    );

    Ok(dest.to_string_lossy().to_string())
}

/// Download PDFs for a list of papers. Prints a summary at the end.
pub async fn download_papers(papers: &[Paper], output_dir: &Path) -> Result<()> {
    if papers.is_empty() {
        println!("{}", "No papers to download.".yellow());
        return Ok(());
    }

    println!(
        "\n{} {} paper(s) to {}",
        "Downloading".green().bold(),
        papers.len(),
        output_dir.display()
    );

    let mut ok_count = 0usize;
    let mut fail_count = 0usize;

    for paper in papers {
        match download_paper(paper, output_dir).await {
            Ok(_) => ok_count += 1,
            Err(e) => {
                eprintln!("  {} {}: {}", "Error".red(), paper.title, e);
                fail_count += 1;
            }
        }
    }

    println!(
        "\n{}: {} succeeded, {} failed",
        "Done".green().bold(),
        ok_count,
        fail_count
    );

    Ok(())
}

/// Download a paper identified by a raw DOI or arXiv ID.
pub async fn download_by_id(id: &str, output_dir: &Path) -> Result<()> {
    let paper = if looks_like_arxiv(id) {
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
    };

    download_paper(&paper, output_dir).await?;
    Ok(())
}

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

/// Simple heuristic: arXiv IDs look like `2301.00306` or `2301.00306v4`.
fn looks_like_arxiv(id: &str) -> bool {
    let id = id.strip_prefix("arXiv:").unwrap_or(id);
    let parts: Vec<&str> = id.splitn(2, '.').collect();
    if parts.len() != 2 {
        return false;
    }
    parts[0].len() == 4 && parts[0].chars().all(|c| c.is_ascii_digit())
}

fn human_size(bytes: usize) -> String {
    if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}
