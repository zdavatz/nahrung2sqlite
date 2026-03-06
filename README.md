# nahrung2sqlite

Rust CLI that converts a TrustBox Excel file into a SQLite database and deploys it to a remote server.

## Prerequisites

- Rust toolchain (rustc, cargo)
- SSH access to zdavatz@65.109.137.20 (for deployment)
- Input file: `trustbox_2_2_2026.xlsx` in the project root

## Usage

```bash
# Build and run the full pipeline (build, convert, deploy via scp)
make

# Or step by step
cargo build --release
make run
```

See `make help` for all available targets.

## How It Works

1. Reads `trustbox_2_2_2026.xlsx` using the calamine library
2. Creates `nahrung.db` (SQLite) with one table per Excel sheet
3. Row 1 becomes column headers, row 2 (metadata) is skipped, rows 3+ are data
4. Column/table names are sanitized (non-alphanumeric chars become `_`)
5. All values stored as TEXT
6. Deploys `nahrung.db` via `scp` to `zdavatz@65.109.137.20:/var/www/pillbox.oddb.org/`
