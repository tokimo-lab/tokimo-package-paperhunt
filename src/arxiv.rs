use anyhow::{Context, Result};
use async_trait::async_trait;
use quick_xml::events::Event;
use quick_xml::Reader;

use crate::paper::Paper;
use crate::provider::PaperProvider;

const ARXIV_API: &str = "https://export.arxiv.org/api/query";

pub struct ArxivProvider {
    client: reqwest::Client,
}

impl ArxivProvider {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl PaperProvider for ArxivProvider {
    fn name(&self) -> &str {
        "arxiv"
    }

    async fn search(
        &self,
        query: &str,
        limit: usize,
        since: Option<&str>,
        until: Option<&str>,
    ) -> Result<Vec<Paper>> {
        let mut search_query = if query.contains(' ') && !query.contains('"') {
            format!("all:\"{}\"", query)
        } else {
            format!("all:{}", query)
        };

        if since.is_some() || until.is_some() {
            let from = since
                .map(|d| format!("{}0000", d.replace('-', "")))
                .unwrap_or_else(|| "000001010000".to_string());
            let to = until
                .map(|d| format!("{}2359", d.replace('-', "")))
                .unwrap_or_else(|| "999912312359".to_string());
            search_query = format!("{} AND submittedDate:[{} TO {}]", search_query, from, to);
        }

        let resp = self
            .client
            .get(ARXIV_API)
            .query(&[
                ("search_query", search_query.as_str()),
                ("start", "0"),
                ("max_results", &limit.to_string()),
                ("sortBy", "submittedDate"),
                ("sortOrder", "descending"),
            ])
            .send()
            .await
            .context("Failed to query arXiv API")?
            .text()
            .await
            .context("Failed to read arXiv response")?;

        parse_atom_feed(&resp)
    }
}

/// Parse an Atom XML feed from arXiv into `Paper` structs.
fn parse_atom_feed(xml: &str) -> Result<Vec<Paper>> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut papers: Vec<Paper> = Vec::new();

    // State for current entry being parsed
    let mut in_entry = false;
    let mut current_tag = String::new();
    let mut in_author = false;

    // Fields for the current entry
    let mut title = String::new();
    let mut summary = String::new();
    let mut authors: Vec<String> = Vec::new();
    let mut published = String::new();
    let mut entry_id = String::new();
    let mut doi: Option<String> = None;
    let mut pdf_url: Option<String> = None;

    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let local = local_name(e.name().as_ref());
                match local.as_str() {
                    "entry" => {
                        in_entry = true;
                        title.clear();
                        summary.clear();
                        authors.clear();
                        published.clear();
                        entry_id.clear();
                        doi = None;
                        pdf_url = None;
                    }
                    "author" if in_entry => {
                        in_author = true;
                    }
                    "link" if in_entry => {
                        let mut href = String::new();
                        let mut link_title = String::new();
                        for attr in e.attributes().flatten() {
                            let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                            let val = String::from_utf8_lossy(&attr.value).to_string();
                            if key == "href" {
                                href = val;
                            } else if key == "title" {
                                link_title = val;
                            }
                        }
                        if link_title == "pdf" {
                            pdf_url = Some(href);
                        }
                    }
                    "doi" if in_entry => {
                        current_tag = "doi".to_string();
                    }
                    _ if in_entry => {
                        current_tag = local;
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(ref e)) => {
                if !in_entry {
                    continue;
                }
                let text = e.unescape().unwrap_or_default().to_string();
                match current_tag.as_str() {
                    "title" => title.push_str(&text),
                    "summary" => summary.push_str(&text),
                    "name" if in_author => authors.push(text),
                    "published" => published = text,
                    "id" => entry_id = text,
                    "doi" => doi = Some(text),
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                let local = local_name(e.name().as_ref());
                match local.as_str() {
                    "entry" => {
                        // Extract arXiv ID from the entry URL
                        let arxiv_id = entry_id.rsplit('/').next().map(|s| s.to_string());

                        // Build PDF url if not found from link
                        if pdf_url.is_none() {
                            if let Some(ref aid) = arxiv_id {
                                pdf_url = Some(format!("https://arxiv.org/pdf/{}", aid));
                            }
                        }

                        let pub_date = if published.len() >= 10 {
                            Some(published[..10].to_string())
                        } else {
                            None
                        };

                        // Normalise whitespace in title/summary
                        let title_clean = title.split_whitespace().collect::<Vec<_>>().join(" ");
                        let summary_clean =
                            summary.split_whitespace().collect::<Vec<_>>().join(" ");

                        papers.push(Paper {
                            title: title_clean,
                            authors: authors.clone(),
                            abstract_text: summary_clean,
                            published_date: pub_date,
                            doi: doi.clone(),
                            arxiv_id,
                            pdf_url: pdf_url.clone(),
                            source: "arxiv".to_string(),
                            citation_count: None,
                        });

                        in_entry = false;
                    }
                    "author" => in_author = false,
                    _ => {}
                }
                current_tag.clear();
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(anyhow::anyhow!("Error parsing arXiv XML: {}", e)),
            _ => {}
        }
        buf.clear();
    }

    Ok(papers)
}

/// Strip namespace prefix from an XML element name (e.g. `arxiv:doi` -> `doi`).
fn local_name(name: &[u8]) -> String {
    let full = String::from_utf8_lossy(name);
    if let Some(pos) = full.find(':') {
        full[pos + 1..].to_string()
    } else {
        full.to_string()
    }
}
