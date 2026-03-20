use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use colored::Colorize;
use futures::StreamExt;

use paperhunt::paper::print_results;
use paperhunt::{download, search_stream, Paper};

#[derive(Parser)]
#[command(name = "paperhunt", version, about = "Search and download academic papers")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Search for academic papers
    Search {
        /// Search terms
        query: String,

        /// Data source
        #[arg(short, long, default_value = "all", value_enum)]
        source: Source,

        /// Maximum results per source
        #[arg(short, long, default_value_t = 10)]
        limit: usize,

        /// Filter papers published since this date (YYYY-MM-DD)
        #[arg(long)]
        since: Option<String>,

        /// Filter papers published until this date (YYYY-MM-DD)
        #[arg(long)]
        until: Option<String>,
    },

    /// Download paper PDFs
    Download {
        /// DOI or arXiv ID to download directly
        id: Option<String>,

        /// Search first, then download all results
        #[arg(long)]
        from_search: Option<String>,

        /// Data source for --from-search
        #[arg(short, long, default_value = "all", value_enum)]
        source: Source,

        /// Max papers for --from-search
        #[arg(short, long, default_value_t = 10)]
        limit: usize,

        /// Output directory
        #[arg(short, long, default_value = "./papers")]
        output: PathBuf,
    },
}

#[derive(Clone, ValueEnum)]
enum Source {
    Arxiv,
    SemanticScholar,
    Openalex,
    All,
}

fn main() {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    if let Err(e) = rt.block_on(async_main()) {
        eprintln!("{} {:#}", "Error:".red().bold(), e);
        std::process::exit(1);
    }
}

async fn async_main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Search {
            query,
            source,
            limit,
            since,
            until,
        } => {
            let papers = do_search(
                &query,
                &source,
                limit,
                since.as_deref(),
                until.as_deref(),
            )
            .await?;
            print_results(&papers);
        }

        Commands::Download {
            id,
            from_search,
            source,
            limit,
            output,
        } => {
            if let Some(query) = from_search {
                let papers = do_search(&query, &source, limit, None, None).await?;
                print_results(&papers);
                download::download_papers(&papers, &output).await?;
            } else if let Some(id) = id {
                download::download_by_id(&id, &output).await?;
            } else {
                anyhow::bail!("Provide a DOI/arXiv ID or use --from-search <query>");
            }
        }
    }

    Ok(())
}

async fn do_search(
    query: &str,
    source: &Source,
    limit: usize,
    since: Option<&str>,
    until: Option<&str>,
) -> Result<Vec<Paper>> {
    println!("{}", "Searching...".dimmed());

    let mut stream = search_stream(
        query,
        limit,
        since.map(|s| s.to_string()),
        until.map(|s| s.to_string()),
    );

    let mut all: Vec<Paper> = Vec::new();
    while let Some(p) = stream.next().await {
        all.push(p);
    }

    // Filter by source if not "all"
    let all = match source {
        Source::Arxiv => all.into_iter().filter(|p| p.source == "arxiv").collect(),
        Source::SemanticScholar => all
            .into_iter()
            .filter(|p| p.source == "semantic_scholar")
            .collect(),
        Source::Openalex => all
            .into_iter()
            .filter(|p| p.source == "openalex")
            .collect(),
        Source::All => all,
    };

    Ok(all)
}
