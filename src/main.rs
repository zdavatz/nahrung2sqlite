use anyhow::{Context, Result};
use calamine::{open_workbook, Reader, Xlsx};
use reqwest::blocking::Client;
use rusqlite::{params, Connection};
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::process::Command;

const OUTPUT_FILE: &str = "nahrung.db";
const XLSX_FILE: &str = "trustbox_2_2_2026.xlsx";
const API_BASE_URL: &str = "https://trustbox.firstbase.ch/api/v1";
const CHUNK_SIZE: u32 = 100;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let use_api = args.iter().any(|a| a == "--api");

    if Path::new(OUTPUT_FILE).exists() {
        fs::remove_file(OUTPUT_FILE).context("Failed to remove existing database")?;
    }

    let conn = Connection::open(OUTPUT_FILE).context("Failed to create SQLite database")?;

    if use_api {
        println!("Fetching data from TrustBox API...");
        fetch_from_api(&conn)?;
    } else {
        println!("Converting {} to {}", XLSX_FILE, OUTPUT_FILE);
        fetch_from_xlsx(&conn)?;
    }

    println!("Database created successfully: {}", OUTPUT_FILE);

    println!("\nCopying database to remote server...");
    copy_to_remote(OUTPUT_FILE)?;

    println!("Done!");
    Ok(())
}

// --- API mode ---

fn fetch_from_api(conn: &Connection) -> Result<()> {
    let user = std::env::var("TRUSTBOX_USER")
        .context("TRUSTBOX_USER environment variable not set")?;
    let password = std::env::var("TRUSTBOX_PASSWORD")
        .context("TRUSTBOX_PASSWORD environment variable not set")?;

    let client = Client::new();

    // First pass: fetch all items to discover all column names and collect data
    let mut all_items: Vec<Value> = Vec::new();
    let mut cursor: Option<String> = None;

    loop {
        let url = format!("{}/items?count={}", API_BASE_URL, CHUNK_SIZE);
        let mut request = client
            .get(&url)
            .basic_auth(&user, Some(&password));

        if let Some(ref c) = cursor {
            request = request.header("x-item-cursor", c);
        }

        let response = request.send().context("API request failed")?;

        let new_cursor = response
            .headers()
            .get("x-item-cursor")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let body: Value = response.json().context("Failed to parse JSON response")?;

        let items = match &body {
            Value::Array(arr) => arr.clone(),
            Value::Object(obj) => {
                // Try common wrapper keys
                if let Some(Value::Array(arr)) = obj.get("items") {
                    arr.clone()
                } else if let Some(Value::Array(arr)) = obj.get("data") {
                    arr.clone()
                } else {
                    vec![body.clone()]
                }
            }
            _ => Vec::new(),
        };

        if items.is_empty() {
            break;
        }

        println!("  Fetched {} items (total: {})", items.len(), all_items.len() + items.len());
        all_items.extend(items);

        match new_cursor {
            Some(c) if !c.is_empty() => cursor = Some(c),
            _ => break,
        }
    }

    if all_items.is_empty() {
        anyhow::bail!("No items received from API");
    }

    println!("  Total items fetched: {}", all_items.len());

    // Discover all column names across all items
    let mut columns = BTreeSet::new();
    for item in &all_items {
        if let Value::Object(map) = item {
            for key in map.keys() {
                columns.insert(key.clone());
            }
        }
    }

    let headers: Vec<String> = columns.into_iter().map(|c| sanitize_column_name(&c)).collect();
    let raw_keys: Vec<String> = {
        let mut keys = BTreeSet::new();
        for item in &all_items {
            if let Value::Object(map) = item {
                for key in map.keys() {
                    keys.insert(key.clone());
                }
            }
        }
        keys.into_iter().collect()
    };

    println!("  Found {} columns", headers.len());

    create_table(conn, "Items", &headers)?;

    let mut inserted = 0;
    for item in &all_items {
        if let Value::Object(map) = item {
            let values: Vec<String> = raw_keys
                .iter()
                .map(|key| match map.get(key) {
                    Some(Value::String(s)) => s.clone(),
                    Some(Value::Number(n)) => n.to_string(),
                    Some(Value::Bool(b)) => b.to_string(),
                    Some(Value::Null) | None => String::new(),
                    Some(other) => other.to_string(),
                })
                .collect();

            if let Err(e) = insert_values(conn, "Items", &headers, &values) {
                eprintln!("  Warning: Failed to insert item: {}", e);
            } else {
                inserted += 1;
            }
        }
    }

    println!("  Successfully inserted {} rows", inserted);
    Ok(())
}

// --- XLSX mode ---

fn fetch_from_xlsx(conn: &Connection) -> Result<()> {
    let mut workbook: Xlsx<_> =
        open_workbook(XLSX_FILE).context("Failed to open Excel file")?;

    for sheet_name in workbook.sheet_names().to_owned() {
        println!("Processing sheet: {}", sheet_name);

        if let Ok(range) = workbook.worksheet_range(&sheet_name) {
            process_sheet(conn, &sheet_name, &range)?;
        }
    }

    Ok(())
}

fn process_sheet(
    conn: &Connection,
    sheet_name: &str,
    range: &calamine::Range<calamine::Data>,
) -> Result<()> {
    let rows: Vec<_> = range.rows().collect();

    if rows.is_empty() {
        println!("  Skipping empty sheet");
        return Ok(());
    }

    let headers: Vec<String> = rows[0]
        .iter()
        .map(|cell| sanitize_column_name(&cell.to_string()))
        .collect();

    if headers.is_empty() {
        println!("  Skipping sheet with no headers");
        return Ok(());
    }

    println!("  Found {} columns", headers.len());

    let table_name = sanitize_table_name(sheet_name);
    create_table(conn, &table_name, &headers)?;

    // Skip header row (index 0) and the second row which is metadata
    let data_rows = &rows[2..];
    println!("  Inserting {} rows", data_rows.len());

    let mut inserted = 0;
    for (idx, row) in data_rows.iter().enumerate() {
        let values: Vec<String> = row
            .iter()
            .map(|cell| match cell {
                calamine::Data::Empty => String::new(),
                calamine::Data::String(s) => s.clone(),
                calamine::Data::Float(f) => f.to_string(),
                calamine::Data::Int(i) => i.to_string(),
                calamine::Data::Bool(b) => b.to_string(),
                calamine::Data::Error(e) => format!("ERROR: {:?}", e),
                calamine::Data::DateTime(dt) => format!("{}", dt),
                calamine::Data::DateTimeIso(s) => s.clone(),
                calamine::Data::DurationIso(s) => s.clone(),
            })
            .collect();

        let mut padded = values;
        padded.resize(headers.len(), String::new());
        padded.truncate(headers.len());

        if let Err(e) = insert_values(conn, &table_name, &headers, &padded) {
            eprintln!("  Warning: Failed to insert row {}: {}", idx + 3, e);
        } else {
            inserted += 1;
        }
    }

    println!("  Successfully inserted {} rows", inserted);
    Ok(())
}

// --- Shared helpers ---

fn sanitize_table_name(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
        .collect()
}

fn sanitize_column_name(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
        .collect();

    if sanitized.chars().next().map_or(false, |c| c.is_numeric()) {
        format!("col_{}", sanitized)
    } else if sanitized.is_empty() {
        "col_unnamed".to_string()
    } else {
        sanitized
    }
}

fn create_table(conn: &Connection, table_name: &str, headers: &[String]) -> Result<()> {
    let columns = headers
        .iter()
        .map(|h| format!("\"{}\" TEXT", h))
        .collect::<Vec<_>>()
        .join(", ");

    let sql = format!("CREATE TABLE IF NOT EXISTS \"{}\" ({})", table_name, columns);
    conn.execute(&sql, params![])
        .context("Failed to create table")?;
    Ok(())
}

fn insert_values(
    conn: &Connection,
    table_name: &str,
    headers: &[String],
    values: &[String],
) -> Result<()> {
    let placeholders = vec!["?"; headers.len()].join(", ");
    let column_names = headers
        .iter()
        .map(|h| format!("\"{}\"", h))
        .collect::<Vec<_>>()
        .join(", ");

    let sql = format!(
        "INSERT INTO \"{}\" ({}) VALUES ({})",
        table_name, column_names, placeholders
    );

    let mut stmt = conn.prepare(&sql)?;
    let params: Vec<&dyn rusqlite::ToSql> = values
        .iter()
        .map(|s| s as &dyn rusqlite::ToSql)
        .collect();
    stmt.execute(&params[..])?;
    Ok(())
}

fn copy_to_remote(file_path: &str) -> Result<()> {
    let remote_user = "zdavatz";
    let remote_host = "65.109.137.20";
    let remote_path = "/var/www/pillbox.oddb.org/";
    let remote_full = format!("{}@{}:{}", remote_user, remote_host, remote_path);

    println!("Executing: scp {} {}", file_path, remote_full);

    let output = Command::new("scp")
        .arg(file_path)
        .arg(&remote_full)
        .output()
        .context("Failed to execute scp command")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("scp failed: {}", stderr);
    }

    println!("Successfully copied {} to {}", file_path, remote_full);
    Ok(())
}
