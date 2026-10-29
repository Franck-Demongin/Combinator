# Combinator

Combinator composes PDF boards from pairs of source images and documents.

## Status

Initial skeleton. Implementation of the composition engine is in progress.

## Build

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build
cargo build --features gui
```

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
