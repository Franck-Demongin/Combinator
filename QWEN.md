# Combinator repository notes

## Branch and publishing

- Default branch for this future project is `main`.
- Commit messages must be in English and use Conventional Commits (`feat:`, `fix:`, etc.).
- Never push, tag remotely, or publish without the user's explicit agreement.
- Releases use annotated tags named `vX.Y.Z`.

## Workspace structure

- Racine : `Cargo.toml` est un workspace avec `members = ["crates/*"]`.
- `crates/core` : package `combinator-core`, code metier sans GUI.
- `crates/cli` : package `combinator-cli`, binaires `combinator` et `combinator-gui`.
- GUI : feature `gui`, alias canonique `slint-ui`.
- Toast : `crates/cli/src/gui/toast/` et `crates/cli/ui/toast/toast.slint`.
- Assets, packaging et workflows restent a la racine.

## Build commands

- `cargo fmt --check`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo build -p combinator-cli --locked`
- `cargo build -p combinator-cli --features gui --locked`
- Lancer la GUI : `cargo run -p combinator-cli --bin combinator-gui --features gui`

## Project identity

- Application name: `Combinator`
- APP_ID: `io.github.Franck-Demongin.Combinator`
- Binaries: `combinator` and `combinator-gui`
- Composant Slint principal : `MainWindow`.
- Call `slint::set_xdg_app_id(APP_ID)` after `MainWindow::new()` and check its result.

## Locking

- `Cargo.lock` is committed.
- CI and release builds use `--locked`.
EOF
## Ancre locale du projet (générée par le scaffold)
- ROOT_DIR = /home/franck/Dev/Rust/combinator
- Toute lecture ou écriture effectuée par Qwen Code doit utiliser ROOT_DIR comme racine absolue.
- Ne jamais déduire un chemin (/tmp/<nom>, nom de projet, slug, profil) à la place de ROOT_DIR.
