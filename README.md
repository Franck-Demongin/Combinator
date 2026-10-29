# Combinator

Combinator composes PDF boards from pairs of source images and documents.

## Status

Initial skeleton. Implementation of the composition engine is in progress.

## Build

```sh
cargo fmt --check
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo build -p combinator-cli
cargo build -p combinator-cli --features gui
```

## CLI

```sh
combinator --dir ./sources -o report.json
combinator --source a.png b.jpg c.pdf -o report.json
combinator --dir ./sources --dry-run
```

Supported inputs: PNG, JPEG, PDF and HEIC/HEIF. A directory is scanned at its
top level only; nested directories are ignored. The report is written as JSON.

## System dependencies (Ubuntu)

```sh
sudo apt-get install -y \
  libfontconfig1-dev libfreetype6-dev libx11-dev \
  libxkbcommon-dev libwayland-dev desktop-file-utils
```

## Linux installation

```sh
cargo build --release --features gui --bin combinator-gui
./packaging/linux/install-local.sh target/release/combinator-gui
```

## Identity

- Application ID: `io.github.Franck-Demongin.Combinator`
- Binaries: `combinator`, `combinator-gui`
