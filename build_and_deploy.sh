#!/bin/bash
set -e

echo "=== Building nahrung2sqlite ==="
cargo build --release

echo ""
echo "=== Copying input file ==="
cp trustbox_2_2_2026.xlsx target/release/

echo ""
echo "=== Running conversion ==="
cd target/release
./nahrung2sqlite

echo ""
echo "=== Deployment complete ==="
echo "Database available at: zdavatz@65.109.137.20:/var/www/pillbox.oddb.org/nahrung.db"
