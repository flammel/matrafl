[private]
default:
    just --list --unsorted

dev:
    PORT=8050 DATABASE_URL=target/db.sqlite3 ASSETS_PATH=assets watchexec -r cargo run

format:
    cargo fmt

check:
    cargo clippy

fix:
    cargo clippy --fix

build:
    cargo build --release

create-user username:
    DATABASE_URL=target/db.sqlite3 cargo run -- create-user {{username}}