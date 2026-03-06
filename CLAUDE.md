# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Single-binary Rust CLI that converts TrustBox product data into a SQLite database (`nahrung.db`) and deploys it via `scp` to a remote server. Two data source modes: XLSX file or TrustBox REST API.

## Commands

- **Build:** `cargo build --release`
- **Run from XLSX:** `make run` — builds, copies xlsx into target/release, runs conversion + scp deploy
- **Run from API:** `make run-api` — requires `TRUSTBOX_USER` and `TRUSTBOX_PASSWORD` env vars
- **Build + run (XLSX):** `make` (or `make all`)
- **Check:** `cargo check`
- **Test:** `cargo test`
- **Clean:** `make clean`

## Architecture

Single-file project (`src/main.rs`). No modules or library crate.

**Two execution paths selected by `--api` flag:**
- **XLSX mode** (`fetch_from_xlsx`): calamine reads Excel → `process_sheet` per sheet → row 1 headers, skip row 2, insert rows 3+
- **API mode** (`fetch_from_api`): reqwest + Basic Auth → cursor-based pagination via `x-item-cursor` header → discovers columns from JSON keys → inserts into `Items` table

**Shared helpers:** `create_table`, `insert_values`, `sanitize_column_name`, `sanitize_table_name`, `copy_to_remote`

**Dependencies:** calamine (Excel), rusqlite with `bundled` (SQLite), reqwest with `blocking`+`json` (HTTP), serde_json (JSON parsing), anyhow (errors).

## Important Details

- XLSX filename hardcoded: `trustbox_2_2_2026.xlsx`
- Output filename hardcoded: `nahrung.db`
- API base URL: `https://trustbox.firstbase.ch/api/v1`
- API credentials via env vars: `TRUSTBOX_USER`, `TRUSTBOX_PASSWORD`
- API pagination chunk size: 100 items per request
- Remote deploy target hardcoded in `main.rs` and `Makefile`
- All SQLite columns are TEXT type regardless of source data
