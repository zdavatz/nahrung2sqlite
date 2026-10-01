# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Single-binary Rust CLI that converts TrustBox product data into a SQLite database (`nahrung.db`) and deploys it via `scp` to a remote server. Two data source modes: XLSX file or TrustBox REST API.

## Commands

- **Build:** `cargo build --release`
- **Run from XLSX:** `make run` — builds, copies xlsx into target/release, runs conversion + scp deploy
- **Run from API:** `make run-api` — requires `TRUSTBOX_USER` and `TRUSTBOX_PASSWORD` env vars
- **Build + run (XLSX):** `make` (or `make all`)
- **Local build, no deploy:** `./target/release/nahrung2sqlite --xlsx "xlsx/<dump>.xlsx" --out db/nahrung.db --no-deploy`
- **Check:** `cargo check`
- **Test:** `cargo test`
- **Clean:** `make clean`

## Architecture

Single-file project (`src/main.rs`). No modules or library crate.

**Two execution paths selected by `--api` flag:**
- **XLSX mode** (`fetch_from_xlsx`): calamine reads Excel → `process_sheet` per sheet → row 1 headers, skip row 2, insert rows 3+
- **API mode** (`fetch_from_api`): reqwest + Basic Auth → cursor-based pagination via `x-item-cursor` header → discovers columns from JSON keys → inserts into `Items` table

**Shared helpers:** `create_table`, `insert_values`, `sanitize_column_name`, `sanitize_table_name`, `copy_to_remote`, `arg_value`

**Dependencies:** calamine (Excel), rusqlite with `bundled` (SQLite), reqwest with `blocking`+`json`+`rustls-tls` (HTTP), serde_json (JSON parsing), anyhow (errors).

## CLI Flags

`arg_value()` parses `--flag <value>` pairs off `std::env::args()`; all flags are optional and the no-flag behaviour is the historical one.

- `--api` — read from the TrustBox REST API instead of XLSX
- `--xlsx <file>` — input workbook (default `trustbox_2_2_2026.xlsx`)
- `--out <path>` — output database path; parent dirs are created via `fs::create_dir_all` (default `nahrung.db`)
- `--no-deploy` — skip the `scp` deploy, build locally only

Convention: input workbooks in `xlsx/`, generated databases in `db/`. Both are covered by the existing `.gitignore` patterns (`*.xlsx`, `*.db`, `nahrung.db`) — the dumps are customer data and must never be committed.

## Important Details

- Default XLSX filename: `trustbox_2_2_2026.xlsx` (override with `--xlsx`)
- Default output filename: `nahrung.db` (override with `--out`)
- API base URL: `https://trustbox.firstbase.ch/api/v1`
- API credentials via env vars: `TRUSTBOX_USER`, `TRUSTBOX_PASSWORD`
- API pagination chunk size: 100 items per request
- Remote deploy target hardcoded in `main.rs` and `Makefile`; suppress with `--no-deploy`
- The deploy is a plain non-interactive `scp` that overwrites the live `nahrung.db`: it needs an SSH key authorised on the server and the host key in `~/.ssh/known_hosts`, otherwise it fails with `Host key verification failed` after the database has already been built
- `reqwest` uses `rustls-tls` with `default-features = false` — the previous native-tls default needed system OpenSSL + `pkg-config`, which broke the build on machines without them
- All SQLite columns are TEXT type regardless of source data

## Data Quality (TrustBox dump)

The dump is **not unique by GTIN**. Measured on the 30.09.2026 dump (97'921 rows, 74 columns;
95'973 distinct GTINs, 97'491 distinct GTIN + GLN pairs, 462 GLNs):

- 1'392 GTINs occur more than once (2'910 rows); in every case same target market (756) but **different GLN** — the same GTIN is published by several data owners
- 111 of those appear as both `GDSNBaseUnit` and `GDSNPackage`
- 430 rows have no GTIN — these are not articles but 353 `Participant` rows, 4 `SubscriptionGDSN` rows and 73 rows without a category; 3 rows carry the placeholder `00000000000000`

Compared with the 31.07.2026 dump (96'286 rows, same 74 columns, 1'302 duplicate GTINs): +1'635
rows, made up of 2'169 new and 537 removed GTIN + GLN pairs.

This is **by design, not a data defect**. Per GS1 Switzerland (19.08.2026): the unique key of a
GDSN item is **GTIN + sender GLN + target market**, and TrustBox follows the same GDSN rules as
firstbase. Since TrustBox is Switzerland-only, the target market is always 756, so GTIN + GLN is
the effective key — the same article may legitimately be published by several suppliers.

Consequence for lookups: **GTIN alone is not a primary key.** A match should return all rows and
expose the GLN. If exactly one record must be picked, there is no authoritative tie-break in the
data — the GS1 company prefix of the GTIN identifies the brand owner and matches a supplier GLN in
only ~26 % of the duplicate cases (measured on a 7-digit prefix: 368 of 1'392 unique, 1'019 with
no prefix match), and there is no last-changed column to fall back on.
