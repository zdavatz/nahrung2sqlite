# nahrung2sqlite

Rust CLI that converts TrustBox product data into a SQLite database and deploys it to a remote server. Supports two data sources: Excel file (XLSX) or the TrustBox REST API.

## Prerequisites

- Rust toolchain (rustc, cargo)
- SSH access to zdavatz@65.109.137.20 (only when deploying)
- **XLSX mode:** an input `.xlsx` (default `trustbox_2_2_2026.xlsx` in the project root, override with `--xlsx`)
- **API mode:** `TRUSTBOX_USER` and `TRUSTBOX_PASSWORD` environment variables

## Usage

```bash
# From Excel file (default)
make

# From TrustBox API
TRUSTBOX_USER=myuser TRUSTBOX_PASSWORD=mypass make run-api
```

See `make help` for all available targets.

### Flags

| Flag | Meaning |
| --- | --- |
| `--api` | Read from the TrustBox REST API instead of an XLSX file |
| `--xlsx <file>` | Input workbook (default `trustbox_2_2_2026.xlsx`) |
| `--out <path>` | Output database path; parent directories are created (default `nahrung.db`) |
| `--no-deploy` | Build the database locally only, skip the `scp` deploy |

Local build from a dated dump, without touching the remote server:

```bash
cargo build --release
./target/release/nahrung2sqlite \
  --xlsx "xlsx/trustbox_Dump, 31.07.2026.xlsx" \
  --out db/nahrung.db \
  --no-deploy
```

By convention input workbooks live in `xlsx/` and generated databases in `db/`; both are gitignored.

## How It Works

### XLSX mode (default)
1. Reads the input workbook (default `trustbox_2_2_2026.xlsx`, or `--xlsx <file>`) using the calamine library
2. Creates the SQLite database (default `nahrung.db`, or `--out <path>`) with one table per Excel sheet
3. Row 1 becomes column headers, row 2 (metadata) is skipped, rows 3+ are data

### API mode (`--api`)
1. Fetches all items from `https://trustbox.firstbase.ch/api/v1` using Basic Auth
2. Uses cursor-based pagination to retrieve items in chunks
3. Discovers columns dynamically from JSON response keys
4. Creates the database with an `Items` table

### Both modes
- Column/table names are sanitized (non-alphanumeric chars become `_`)
- All values stored as TEXT
- Deploys the database via `scp` to `zdavatz@65.109.137.20:/var/www/pillbox.oddb.org/`, unless `--no-deploy` is given

## Data notes

The TrustBox dump is not unique by GTIN. In the 31.07.2026 dump (96'286 rows, 74 columns):

- 1'302 GTINs occur more than once (2'730 rows). In every case the target market is identical (756) but the GLN differs — the same GTIN is published by several data owners.
- 112 of those carry the same GTIN once as `GDSNBaseUnit` and once as `GDSNPackage`.
- 427 rows have no GTIN at all, 3 carry the placeholder GTIN `00000000000000`.

So a lookup by GTIN alone can return several rows; pick the authoritative record by GLN (or filter placeholders) when matching against a customer GTIN list.
