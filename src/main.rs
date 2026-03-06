use anyhow::{Context, Result};
use calamine::{open_workbook, Reader, Xlsx};
use rusqlite::{params, Connection};
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() -> Result<()> {
    let input_file = "trustbox_2_2_2026.xlsx";
    let output_file = "nahrung.db";
    
    println!("Converting {} to {}", input_file, output_file);
    
    // Remove existing database if it exists
    if Path::new(output_file).exists() {
        fs::remove_file(output_file)
            .context("Failed to remove existing database")?;
    }
    
    // Open Excel file
    let mut workbook: Xlsx<_> = open_workbook(input_file)
        .context("Failed to open Excel file")?;
    
    // Create SQLite database
    let conn = Connection::open(output_file)
        .context("Failed to create SQLite database")?;
    
    // Process each sheet
    for sheet_name in workbook.sheet_names().to_owned() {
        println!("Processing sheet: {}", sheet_name);
        
        if let Ok(range) = workbook.worksheet_range(&sheet_name) {
            process_sheet(&conn, &sheet_name, &range)?;
        }
    }
    
    println!("Database created successfully: {}", output_file);
    
    // Copy file to remote server
    println!("\nCopying database to remote server...");
    copy_to_remote(output_file)?;
    
    println!("Done!");
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
    
    // First row contains headers
    let headers: Vec<String> = rows[0]
        .iter()
        .map(|cell| sanitize_column_name(&cell.to_string()))
        .collect();
    
    if headers.is_empty() {
        println!("  Skipping sheet with no headers");
        return Ok(());
    }
    
    println!("  Found {} columns", headers.len());
    
    // Create table with sanitized column names
    let table_name = sanitize_table_name(sheet_name);
    create_table(conn, &table_name, &headers)?;
    
    // Skip header row (index 0) and the second row which seems to be metadata
    let data_rows = &rows[2..];
    println!("  Inserting {} rows", data_rows.len());
    
    // Insert data
    let mut inserted = 0;
    for (idx, row) in data_rows.iter().enumerate() {
        if let Err(e) = insert_row(conn, &table_name, &headers, row) {
            eprintln!("  Warning: Failed to insert row {}: {}", idx + 3, e);
        } else {
            inserted += 1;
        }
    }
    
    println!("  Successfully inserted {} rows", inserted);
    Ok(())
}

fn sanitize_table_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn sanitize_column_name(name: &str) -> String {
    let sanitized = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>();
    
    // Ensure column name doesn't start with a number
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
    
    let sql = format!("CREATE TABLE \"{}\" ({})", table_name, columns);
    
    conn.execute(&sql, params![])
        .context("Failed to create table")?;
    
    Ok(())
}

fn insert_row(
    conn: &Connection,
    table_name: &str,
    headers: &[String],
    row: &[calamine::Data],
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
    
    // Convert row data to strings, handling None values
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
    
    // Pad with empty strings if row is shorter than headers
    let mut padded_values = values;
    while padded_values.len() < headers.len() {
        padded_values.push(String::new());
    }
    
    // Truncate if row is longer than headers
    padded_values.truncate(headers.len());
    
    let params: Vec<&dyn rusqlite::ToSql> = padded_values
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
