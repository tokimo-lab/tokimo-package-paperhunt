use serde::{Deserialize, Serialize};

/// Events for streaming paper PDF downloads.
#[derive(Debug, Clone)]
pub enum DownloadEvent {
    /// File metadata (first event)
    FileInfo {
        title: String,
        authors: Vec<String>,
        filename: String,
        total_bytes: Option<u64>,
    },
    /// A chunk of PDF data
    Data { bytes: Vec<u8>, downloaded: u64 },
    /// Download complete
    Done { filename: String, total_bytes: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paper {
    pub title: String,
    pub authors: Vec<String>,
    pub abstract_text: String,
    pub published_date: Option<String>,
    pub doi: Option<String>,
    pub arxiv_id: Option<String>,
    pub pdf_url: Option<String>,
    pub source: String,
    pub citation_count: Option<u32>,
}

impl Paper {
    /// Return a usable identifier string (arXiv ID or DOI).
    pub fn identifier(&self) -> String {
        if let Some(id) = &self.arxiv_id {
            return id.clone();
        }
        if let Some(doi) = &self.doi {
            return doi.clone();
        }
        "(no id)".to_string()
    }

    /// Generate a sanitized filename for the PDF.
    pub fn filename(&self) -> String {
        let year = self
            .published_date
            .as_deref()
            .and_then(|d| d.get(..4))
            .unwrap_or("unknown");

        let title_clean: String = self
            .title
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == ' ' {
                    c
                } else {
                    '_'
                }
            })
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join("_");

        let truncated = if title_clean.len() > 80 {
            &title_clean[..80]
        } else {
            &title_clean
        };

        format!("{}-{}.pdf", year, truncated)
    }
}
