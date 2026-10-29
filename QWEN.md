# Combinator repository notes

## Branch and publishing

- Default branch for this future project is `main`.
- Commit messages must be in English and use Conventional Commits (`feat:`, `fix:`, etc.).
- Never push, tag remotely, or publish without the user's explicit agreement.
- Releases use annotated tags named `vX.Y.Z`.

## Build commands

- `cargo fmt --check`
- `cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo build`
- `cargo build --features gui`

## Project identity

- Application name: `Combinator`
- APP_ID: `io.github.Franck-Demongin.Combinator`
- Binaries: `combinator` and `combinator-gui`
- Call `slint::set_xdg_app_id(APP_ID)` after `AppWindow::new()` and check its result.

## Locking

- `Cargo.lock` is committed.
- CI and release builds use `--locked`.
