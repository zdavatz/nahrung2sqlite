# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Single-binary Rust CLI that converts a TrustBox Excel file (`trustbox_2_2_2026.xlsx`) into a SQLite database (`nahrung.db`) and deploys it via `scp` to a remote server. See README.md for full usage details.

## Commands

- **Build:** `cargo build --release`
- **Run (full pipeline):** `make run` — builds, copies xlsx into target/release, runs conversion + scp deploy
- **Build + run:** `make` (or `make all`)
- **Check:** `cargo check`
- **Test:** `cargo test`
- **Clean:** `make clean`

## Architecture

Single-file project (`src/main.rs`, ~220 lines). No modules or library crate.

**Flow:** Open xlsx (calamine) → create SQLite db (rusqlite with bundled SQLite) → for each sheet: read row 1 as headers, skip row 2 (metadata), insert rows 3+ as TEXT → scp the db to remote server.

**Key functions:**
- `process_sheet` — reads headers, creates table, inserts data rows
- `sanitize_column_name` / `sanitize_table_name` — replace non-alphanumeric chars with `_`; prefix numeric-leading column names with `col_`
- `insert_row` — pads/truncates row values to match header count; all values stored as TEXT
- `copy_to_remote` — shells out to `scp`

**Dependencies:** calamine (Excel reading), rusqlite with `bundled` feature (SQLite), anyhow (error handling), ssh2 (declared but unused — scp is done via `Command`).

## Important Details

- Input filename is hardcoded: `trustbox_2_2_2026.xlsx`
- Output filename is hardcoded: `nahrung.db`
- Remote deploy target is hardcoded in both `main.rs` and `Makefile`
- The `make run` target expects the xlsx file in the project root and copies it to `target/release/` before executing
- All SQLite columns are TEXT type regardless of source data
