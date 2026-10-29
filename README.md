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
combinator --dir ./sources -o report.pdf
combinator --source a.png b.jpg c.pdf -o report.pdf
combinator --dir ./sources --dry-run
combinator --dir ./sources --report report.json
```

Supported inputs: PNG, JPEG, WebP, PDF and HEIC/HEIF. A directory is scanned at its
top level only; nested directories are ignored. The primary output is a composed
A4 PDF. An optional JSON composition report can be written with `--report`.

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
