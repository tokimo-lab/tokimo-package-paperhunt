use anyhow::{Context, Result};
use serde::Deserialize;
use std::thread;
use std::time::Duration;

use crate::paper::Paper;

const SS_API: &str = "https://api.semanticscholar.org/graph/v1/paper/search";

const FIELDS: &str =
    "paperId,externalIds,title,abstract,authors,year,publicationDate,journal,venue,citationCount,openAccessPdf,url";

#[derive(Debug, Deserialize)]
struct SsResponse {
    #[serde(default)]
    total: u64,
    #[allow(dead_code)]
    offset: Option<u64>,
    #[allow(dead_code)]
    next: Option<u64>,
    #[serde(default)]
    data: Vec<SsPaper>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SsPaper {
    #[allow(dead_code)]
    paper_id: Option<String>,
    title: Option<String>,
    #[serde(rename = "abstract")]
    abstract_text: Option<String>,
    #[serde(default)]
    authors: Vec<SsAuthor>,
    publication_date: Option<String>,
    external_ids: Option<SsExternalIds>,
    open_access_pdf: Option<SsOpenAccessPdf>,
    citation_count: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct SsAuthor {
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SsExternalIds {
    #[serde(rename = "DOI")]
    doi: Option<String>,
    #[serde(rename = "ArXiv")]
    arxiv_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SsOpenAccessPdf {
    url: Option<String>,
}

/// Search Semantic Scholar for papers matching `query`.
pub fn search(
    query: &str,
    limit: usize,
    since: Option<&str>,
    until: Option<&str>,
) -> Result<Vec<Paper>> {
    let client = reqwest::blocking::Client::new();

    let date_range = match (since, until) {
        (Some(s), Some(u)) => Some(format!("{}:{}", s, u)),
        (Some(s), None) => Some(format!("{}:", s)),
        (None, Some(u)) => Some(format!(":{}", u)),
        _ => None,
    };

    let effective_limit = limit.min(100);

    let mut params: Vec<(&str, String)> = vec![
        ("query", query.to_string()),
        ("fields", FIELDS.to_string()),
        ("limit", effective_limit.to_string()),
        ("offset", "0".to_string()),
    ];

    if let Some(ref dr) = date_range {
        params.push(("publicationDateOrYear", dr.clone()));
    }

    let resp = client
        .get(SS_API)
        .header("User-Agent", "paperhunt/0.1.0 (academic-paper-search-cli)")
        .query(&params)
        .send()
        .context("Failed to query Semantic Scholar API")?;

    let status = resp.status();
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        // Retry with backoff
        for attempt in 1..=3 {
            let wait = Duration::from_secs(5 * attempt);
            eprintln!("  Rate limited, retrying in {}s...", wait.as_secs());
            thread::sleep(wait);
            let retry_resp = client
                .get(SS_API)
                .header("User-Agent", "paperhunt/0.1.0 (academic-paper-search-cli)")
                .query(&params)
                .send()
                .context("Failed to query Semantic Scholar API")?;
            if retry_resp.status().is_success() {
                let ss_resp: SsResponse = retry_resp
                    .json()
                    .context("Failed to parse Semantic Scholar response")?;
                return Ok(convert_papers(ss_resp));
            }
            if retry_resp.status() != reqwest::StatusCode::TOO_MANY_REQUESTS {
                let body = retry_resp.text().unwrap_or_default();
                anyhow::bail!("Semantic Scholar API returned HTTP {}: {}", status, body);
            }
        }
        anyhow::bail!("Semantic Scholar API rate limit exceeded after retries");
    }
    if !status.is_success() {
        let body = resp.text().unwrap_or_default();
        anyhow::bail!(
            "Semantic Scholar API returned HTTP {}: {}",
            status,
            body
        );
    }

    let ss_resp: SsResponse = resp
        .json()
        .context("Failed to parse Semantic Scholar response")?;

    Ok(convert_papers(ss_resp))
}

fn convert_papers(ss_resp: SsResponse) -> Vec<Paper> {
    if ss_resp.total == 0 {
        return Vec::new();
    }

    ss_resp
        .data
        .into_iter()
        .map(|p| {
            let authors = p
                .authors
                .iter()
                .filter_map(|a| a.name.clone())
                .collect();

            let doi = p.external_ids.as_ref().and_then(|e| e.doi.clone());
            let arxiv_id = p.external_ids.as_ref().and_then(|e| e.arxiv_id.clone());

            let pdf_url = p
                .open_access_pdf
                .and_then(|oa| oa.url)
                .or_else(|| {
                    arxiv_id
                        .as_ref()
                        .map(|id| format!("https://arxiv.org/pdf/{}", id))
                });

            Paper {
                title: p.title.unwrap_or_default(),
                authors,
                abstract_text: p.abstract_text.unwrap_or_default(),
                published_date: p.publication_date,
                doi,
                arxiv_id,
                pdf_url,
                source: "semantic_scholar".to_string(),
                citation_count: p.citation_count,
            }
        })
        .collect()
}
