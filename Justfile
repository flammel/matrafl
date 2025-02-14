set dotenv-load

[private]
default:
    just --list --unsorted

dev:
    PORT=8050 DATABASE_URL=target/db.sqlite3 ASSETS_PATH=assets bacon serve

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

deploy: build
    rsync --archive --verbose --progress --compress ./target/release/matrafl $DEPLOY_USER@$DEPLOY_HOST:$DEPLOY_PATH/bin/matrafl.tmp
    rsync --archive --verbose --progress --compress ./assets/ $DEPLOY_USER@$DEPLOY_HOST:$DEPLOY_PATH/assets
    ssh $DEPLOY_USER@$DEPLOY_HOST "mv $DEPLOY_PATH/bin/matrafl.tmp $DEPLOY_PATH/bin/matrafl"