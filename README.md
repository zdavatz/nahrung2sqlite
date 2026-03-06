# nahrung2sqlite

Rust CLI that converts TrustBox product data into a SQLite database and deploys it to a remote server. Supports two data sources: Excel file (XLSX) or the TrustBox REST API.

## Prerequisites

- Rust toolchain (rustc, cargo)
- SSH access to zdavatz@65.109.137.20 (for deployment)
- **XLSX mode:** `trustbox_2_2_2026.xlsx` in the project root
- **API mode:** `TRUSTBOX_USER` and `TRUSTBOX_PASSWORD` environment variables

## Usage

```bash
# From Excel file (default)
make

# From TrustBox API
TRUSTBOX_USER=myuser TRUSTBOX_PASSWORD=mypass make run-api
```

See `make help` for all available targets.

## How It Works

### XLSX mode (default)
1. Reads `trustbox_2_2_2026.xlsx` using the calamine library
2. Creates `nahrung.db` (SQLite) with one table per Excel sheet
3. Row 1 becomes column headers, row 2 (metadata) is skipped, rows 3+ are data

### API mode (`--api`)
1. Fetches all items from `https://trustbox.firstbase.ch/api/v1` using Basic Auth
2. Uses cursor-based pagination to retrieve items in chunks
3. Discovers columns dynamically from JSON response keys
4. Creates `nahrung.db` with an `Items` table

### Both modes
- Column/table names are sanitized (non-alphanumeric chars become `_`)
- All values stored as TEXT
- Deploys `nahrung.db` via `scp` to `zdavatz@65.109.137.20:/var/www/pillbox.oddb.org/`
