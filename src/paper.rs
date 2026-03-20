use colored::Colorize;

#[derive(Debug, Clone)]
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
            .map(|c| if c.is_alphanumeric() || c == ' ' { c } else { '_' })
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

/// Print a list of papers as a formatted table.
pub fn print_results(papers: &[Paper]) {
    if papers.is_empty() {
        println!("{}", "No results found.".yellow());
        return;
    }

    println!(
        "\n{} {}",
        "Found".green().bold(),
        format!("{} paper(s):", papers.len()).green().bold()
    );
    println!("{}", "─".repeat(100).dimmed());

    for (i, paper) in papers.iter().enumerate() {
        let idx = format!("[{}]", i + 1).cyan().bold();
        let title = paper.title.bold();
        println!("{} {}", idx, title);

        let authors_str = if paper.authors.len() > 3 {
            format!(
                "{}, et al.",
                paper.authors[..3].join(", ")
            )
        } else {
            paper.authors.join(", ")
        };
        println!("    {} {}", "Authors:".dimmed(), authors_str);

        if let Some(date) = &paper.published_date {
            println!("    {} {}", "Date:".dimmed(), date);
        }

        let id_label = match paper.source.as_str() {
            "arxiv" => "arXiv ID:",
            _ => "Paper ID:",
        };
        println!("    {} {}", id_label.dimmed(), paper.identifier().yellow());

        if let Some(doi) = &paper.doi {
            println!("    {} {}", "DOI:".dimmed(), doi);
        }

        if let Some(count) = paper.citation_count {
            println!("    {} {}", "Citations:".dimmed(), count);
        }

        let source_colored = match paper.source.as_str() {
            "arxiv" => paper.source.red(),
            "semantic_scholar" => paper.source.blue(),
            "openalex" => paper.source.magenta(),
            _ => paper.source.normal(),
        };
        println!("    {} {}", "Source:".dimmed(), source_colored);

        if i < papers.len() - 1 {
            println!("{}", "─".repeat(100).dimmed());
        }
    }

    println!("{}", "─".repeat(100).dimmed());
}
