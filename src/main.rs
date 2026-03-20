mod arxiv;
mod download;
mod openalex;
mod paper;
mod semantic_scholar;

use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use colored::Colorize;

use paper::print_results;

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
    let cli = Cli::parse();

    if let Err(e) = run(cli) {
        eprintln!("{} {:#}", "Error:".red().bold(), e);
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {
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
            )?;
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
                let papers = do_search(&query, &source, limit, None, None)?;
                print_results(&papers);
                download::download_papers(&papers, &output)?;
            } else if let Some(id) = id {
                download::download_by_id(&id, &output)?;
            } else {
                anyhow::bail!("Provide a DOI/arXiv ID or use --from-search <query>");
            }
        }
    }

    Ok(())
}

fn do_search(
    query: &str,
    source: &Source,
    limit: usize,
    since: Option<&str>,
    until: Option<&str>,
) -> Result<Vec<paper::Paper>> {
    let mut papers = Vec::new();

    let search_arxiv = matches!(source, Source::Arxiv | Source::All);
    let search_ss = matches!(source, Source::SemanticScholar | Source::All);
    let search_oa = matches!(source, Source::Openalex | Source::All);

    if search_arxiv {
        println!("{}", "Searching arXiv...".dimmed());
        match arxiv::search(query, limit, since, until) {
            Ok(mut results) => papers.append(&mut results),
            Err(e) => eprintln!("{} arXiv search failed: {:#}", "Warning:".yellow(), e),
        }
        if search_ss || search_oa {
            thread::sleep(Duration::from_secs(1));
        }
    }

    if search_oa {
        println!("{}", "Searching OpenAlex...".dimmed());
        match openalex::search(query, limit, since, until) {
            Ok(mut results) => papers.append(&mut results),
            Err(e) => eprintln!("{} OpenAlex search failed: {:#}", "Warning:".yellow(), e),
        }
        if search_ss {
            thread::sleep(Duration::from_secs(1));
        }
    }

    if search_ss {
        println!("{}", "Searching Semantic Scholar...".dimmed());
        match semantic_scholar::search(query, limit, since, until) {
            Ok(mut results) => papers.append(&mut results),
            Err(e) => eprintln!(
                "{} Semantic Scholar search failed: {:#}",
                "Warning:".yellow(),
                e
            ),
        }
    }

    Ok(papers)
}
