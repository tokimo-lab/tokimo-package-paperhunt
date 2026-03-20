use anyhow::{Context, Result};
use chrono;
use serde::Deserialize;

use crate::paper::Paper;

const OPENALEX_API: &str = "https://api.openalex.org/works";

#[derive(Debug, Deserialize)]
struct OaResponse {
    #[allow(dead_code)]
    meta: Option<OaMeta>,
    #[serde(default)]
    results: Vec<OaWork>,
}

#[derive(Debug, Deserialize)]
struct OaMeta {
    #[allow(dead_code)]
    count: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct OaWork {
    title: Option<String>,
    doi: Option<String>,
    publication_date: Option<String>,
    #[serde(default)]
    authorships: Vec<OaAuthorship>,
    cited_by_count: Option<u32>,
    open_access: Option<OaOpenAccess>,
    #[serde(default)]
    ids: OaIds,
}

#[derive(Debug, Deserialize, Default)]
struct OaIds {
    openalex: Option<String>,
    #[allow(dead_code)]
    doi: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OaAuthorship {
    author: Option<OaAuthor>,
}

#[derive(Debug, Deserialize)]
struct OaAuthor {
    display_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OaOpenAccess {
    oa_url: Option<String>,
}

/// Search OpenAlex for papers matching `query`.
pub fn search(
    query: &str,
    limit: usize,
    since: Option<&str>,
    _until: Option<&str>,
) -> Result<Vec<Paper>> {
    let client = reqwest::blocking::Client::new();

    let effective_limit = limit.min(100);

    let mut filter_parts: Vec<String> = Vec::new();
    if let Some(s) = since {
        filter_parts.push(format!("from_publication_date:{}", s));
    }
    if let Some(u) = _until {
        filter_parts.push(format!("to_publication_date:{}", u));
    } else {
        // Avoid future-dated entries
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        filter_parts.push(format!("to_publication_date:{}", today));
    }

    let mut params: Vec<(&str, String)> = vec![
        ("search", query.to_string()),
        ("per_page", effective_limit.to_string()),
        ("sort", "publication_date:desc".to_string()),
        ("mailto", "paperhunt@example.com".to_string()),
    ];

    if !filter_parts.is_empty() {
        params.push(("filter", filter_parts.join(",")));
    }

    let resp = client
        .get(OPENALEX_API)
        .header("User-Agent", "paperhunt/0.1.0 (academic-paper-search-cli)")
        .query(&params)
        .send()
        .context("Failed to query OpenAlex API")?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().unwrap_or_default();
        anyhow::bail!("OpenAlex API returned HTTP {}: {}", status, body);
    }

    let oa_resp: OaResponse = resp
        .json()
        .context("Failed to parse OpenAlex response")?;

    let papers = oa_resp
        .results
        .into_iter()
        .map(|w| {
            let authors = w
                .authorships
                .iter()
                .filter_map(|a| a.author.as_ref().and_then(|au| au.display_name.clone()))
                .collect();

            let doi = w.doi.as_ref().map(|d| {
                d.strip_prefix("https://doi.org/")
                    .unwrap_or(d)
                    .to_string()
            });

            let pdf_url = w.open_access.and_then(|oa| oa.oa_url);

            Paper {
                title: w.title.unwrap_or_default(),
                authors,
                abstract_text: String::new(),
                published_date: w.publication_date,
                doi,
                arxiv_id: None,
                pdf_url,
                source: "openalex".to_string(),
                citation_count: w.cited_by_count,
            }
        })
        .collect();

    Ok(papers)
}
