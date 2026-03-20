# 🔍 paperhunt

A fast command-line tool for searching and downloading academic papers, written in Rust.

Inspired by [findpapers](https://github.com/jonatasgrosman/findpapers) but reimplemented as a native CLI — no Python runtime, no server, just a single binary.

## Features

- **Multi-source search** — query arXiv, OpenAlex (243M+ papers), and Semantic Scholar from one command
- **PDF downloads** — grab papers directly by arXiv ID or DOI, or batch-download from search results
- **Date filtering** — narrow results with `--since` and `--until` flags
- **Rate-limit handling** — automatic retry with exponential backoff for Semantic Scholar
- **Color-coded output** — source-tagged, readable results in your terminal
- **Fast** — native Rust binary, no interpreter overhead

## Installation

Build from source (requires [Rust](https://www.rust-lang.org/tools/install)):

```sh
git clone <repo-url>
cd paperhunt
cargo build --release
```

The binary will be at `./target/release/paperhunt`.

## Quick Start

Search for papers on a topic:

```sh
paperhunt search "large language model" --source arxiv --limit 5
```

Download a paper by arXiv ID:

```sh
paperhunt download 2603.19225v1 -o ./papers/
```

Search and download in one step:

```sh
paperhunt download --from-search "machine learning" --source arxiv --limit 3 -o ./papers/
```

## Supported Data Sources

| Source | Flag value | Description |
|---|---|---|
| **arXiv** | `arxiv` | Preprint server covering physics, math, CS, and more. Provides direct PDF links. |
| **OpenAlex** | `openalex` | Open catalog of 243M+ scholarly works, authors, and institutions. Includes citation counts and open access URLs. |
| **Semantic Scholar** | `semantic-scholar` | AI-powered research database from the Allen Institute. Provides citation counts and open access PDFs. |
| **All** | `all` (default) | Queries all three sources in a single run. |

## CLI Reference

### `paperhunt search`

Search for academic papers across one or more sources.

```
Usage: paperhunt search [OPTIONS] <QUERY>
```

| Argument / Flag | Description | Default |
|---|---|---|
| `<QUERY>` | Search terms (required) | — |
| `-s, --source <SOURCE>` | Data source: `arxiv`, `semantic-scholar`, `openalex`, `all` | `all` |
| `-l, --limit <LIMIT>` | Maximum results per source | `10` |
| `--since <SINCE>` | Filter papers published on or after this date (`YYYY-MM-DD`) | — |
| `--until <UNTIL>` | Filter papers published on or before this date (`YYYY-MM-DD`) | — |

### `paperhunt download`

Download paper PDFs by ID or from search results.

```
Usage: paperhunt download [OPTIONS] [ID]
```

| Argument / Flag | Description | Default |
|---|---|---|
| `[ID]` | arXiv ID or DOI to download directly | — |
| `--from-search <QUERY>` | Search first, then download all results | — |
| `-s, --source <SOURCE>` | Data source for `--from-search`: `arxiv`, `semantic-scholar`, `openalex`, `all` | `all` |
| `-l, --limit <LIMIT>` | Max papers for `--from-search` | `10` |
| `-o, --output <DIR>` | Output directory for downloaded PDFs | `./papers` |

> Provide either `[ID]` for a single download or `--from-search` for batch downloading — not both.

## Examples

**Search arXiv for recent papers:**

```sh
paperhunt search "transformer" --source arxiv --limit 5 --since 2025-01-01
```

**Search all sources with a date range:**

```sh
paperhunt search "deep learning" --source all --limit 10 --since 2024-01-01 --until 2024-12-31
```

**Download a single paper by arXiv ID:**

```sh
paperhunt download 2603.19225v1 -o ./papers/
```

**Download a paper by DOI:**

```sh
paperhunt download 10.1038/s41586-021-03819-2 -o ./papers/
```

**Search and download from Semantic Scholar:**

```sh
paperhunt download --from-search "reinforcement learning" --source semantic-scholar --limit 5 -o ./papers/
```

## License

[MIT](https://opensource.org/licenses/MIT)
