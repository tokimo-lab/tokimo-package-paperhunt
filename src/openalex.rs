use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono;
use serde::Deserialize;

use crate::paper::Paper;
use crate::provider::PaperProvider;

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

pub struct OpenAlexProvider {
    client: reqwest::Client,
}

impl OpenAlexProvider {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl PaperProvider for OpenAlexProvider {
    fn name(&self) -> &str {
        "openalex"
    }

    async fn search(
        &self,
        query: &str,
        limit: usize,
        since: Option<&str>,
        until: Option<&str>,
    ) -> Result<Vec<Paper>> {
        let effective_limit = limit.min(100);

        let mut filter_parts: Vec<String> = Vec::new();
        if let Some(s) = since {
            filter_parts.push(format!("from_publication_date:{}", s));
        }
        if let Some(u) = until {
            filter_parts.push(format!("to_publication_date:{}", u));
        } else {
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

        let resp = self
            .client
            .get(OPENALEX_API)
            .header("User-Agent", "paperhunt/0.1.0 (academic-paper-search-cli)")
            .query(&params)
            .send()
            .await
            .context("Failed to query OpenAlex API")?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("OpenAlex API returned HTTP {}: {}", status, body);
        }

        let oa_resp: OaResponse = resp
            .json()
            .await
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

                let doi = w
                    .doi
                    .as_ref()
                    .map(|d| d.strip_prefix("https://doi.org/").unwrap_or(d).to_string());

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
}
