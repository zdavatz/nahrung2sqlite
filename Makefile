.PHONY: build run deploy clean all

all: build run

build:
	cargo build --release

run: build
	cp trustbox_2_2_2026.xlsx target/release/
	cd target/release && ./nahrung2sqlite

deploy: run
	@echo "Database deployed to zdavatz@65.109.137.20:/var/www/pillbox.oddb.org/nahrung.db"

clean:
	cargo clean
	rm -f nahrung.db

test:
	cargo test

check:
	cargo check

help:
	@echo "Available targets:"
	@echo "  all     - Build and run (default)"
	@echo "  build   - Build the project"
	@echo "  run     - Build, convert Excel to SQLite, and deploy"
	@echo "  deploy  - Same as run"
	@echo "  clean   - Remove build artifacts"
	@echo "  test    - Run tests"
	@echo "  check   - Check code without building"
