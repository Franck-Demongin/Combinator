# Combinator

Combinator composes PDF boards from pairs of source images and documents.

## Status

Implemented for the 0.1.0 release: the composition engine and CLI are complete for A4 PDF
composition from PNG, JPEG, WebP, PDF, and HEIC/HEIF sources, with optional JSON report
output. The optional Slint GUI MVP is implemented around the same pipeline and provides
native source selection, asynchronous generation, progress feedback, toasts, and
output/report/overwrite controls.

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
  libxkbcommon-dev libwayland-dev desktop-file-utils \
  poppler-utils
```

`poppler-utils` provides `pdftoppm`, used to rasterize PDF source pages.
Equivalent Windows installations must also provide `pdftoppm`.

## Linux installation

```sh
cargo build --release --features gui --bin combinator-gui
./packaging/linux/install-local.sh target/release/combinator-gui
```

## Identity

- Application ID: `io.github.Franck-Demongin.Combinator`
- Binaries: `combinator`, `combinator-gui`
