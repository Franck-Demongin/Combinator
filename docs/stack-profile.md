---
title: Profil technique du projet cible
version: 0.2.2
status: validé
---

# Profil technique — Combinator

> Document de référence pour l'Analyste, le Développeur et le QA.
> Racine absolue du projet cible : `/home/franck/Dev/Rust/combinator` (doit rester identique aux écritures).

## 1. Langage et version

- Langage : Rust
- Version minimale / recommandée : Rust 1.99 (workspace `rust-version = "1.99"`, edition 2024)
- Gestionnaire / runtime : Cargo

## 2. Outils imposés ou préférés

- Gestion des dépendances : Cargo (`Cargo.toml` workspace + crates)
- Exécution : `cargo run -p combinator-cli` (CLI), `cargo run -p combinator-cli --features gui --bin combinator-gui` (GUI)
- Lint : `cargo clippy` (clippy `all` et `pedantic` en `warn` au niveau workspace)
- Formatage : `rustfmt` via `cargo fmt`
- Tests : `cargo test`
- Build : `cargo build`
- Documentation : Markdown sous `docs/`

## 3. Bibliothèques

### Préférées (par défaut)

- `combinator-core` : cœur métier
- `anyhow`, `thiserror` : erreurs
- `tracing` : journalisation (workspace)
- `clap` : CLI
- `image` (png, jpeg, webp) : décodage d'images
- `heic-rs` : décodage HEIC/HEIF (feature `std`)
- `lopdf` : lecture PDF source et génération PDF
- `natord` : tri naturel
- `serde`, `serde_json` : JSON
- `zip` : archives ZIP
- `tempfile` : répertoires temporaires
- `slint` (optionnel) : GUI
- `rfd` (optionnel) : dialogues natifs
- `slint-build` (optionnel, build) : intégration Slint
- `winresource` (Windows, build) : icône exécutable

### Exclues (et pourquoi)

- Slint par défaut sans feature : volontairement désactivé par défaut (`default = []`) pour laisser une CLI légère ; activé via `gui`.
- Récursion répertoire disque pour la découverte : non implémentée (scan niveau supérieur) ; utiliser les archives ZIP pour agréger.

## 4. Structure attendue

```text
.
├── Cargo.toml
├── crates/
│   ├── core/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── composition.rs
│   │       ├── sources.rs
│   │       └── error.rs
│   └── cli/
│       ├── Cargo.toml
│       ├── build.rs
│       ├── src/
│       │   ├── main.rs
│       │   ├── lib.rs
│       │   ├── run.rs
│       │   ├── gui.rs
│       │   └── gui_main.rs
│       ├── ui/
│       └── tests/
├── assets/
├── packaging/linux/
└── docs/
```

## 5. Conventions de code

- Style : formatage Rust standard (`rustfmt`).
- Typage : Rust avec erreur explicite (`thiserror` dans le cœur, `anyhow` en CLI).
- Nommage : modules et fichiers en snake_case ; types en CamelCase.
- Organisation des modules : cœur sans dépendance GUI ; CLI isole `run`, `gui`, points d'entrée.

## 6. Conventions de design / interface

- Interface : CLI et GUI Slint (optionnelle, feature Cargo).
- Bibliothèque visuelle / composants : Slint 1.18 (backend winit, rendu femtovg/software).
- Accessibilité : feature Slint `accessibility` activée.
- Messages et sortie utilisateur : français avec accents corrects ; erreurs explicites.

## 7. Commandes de validation stricte

Depuis la racine du projet `/home/franck/Dev/Rust/combinator` :

- Tests : `cargo test --workspace`
- Lint : `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- Build : `cargo build -p combinator-cli`
- Vérification supplémentaire : `cargo fmt --check` et `cargo build -p combinator-cli --features gui`

## 8. Cas d'usage typiques

- Composition CLI : `cargo run -p combinator-cli -- --dir ./sources -o report.pdf`
- Dry-run : `cargo run -p combinator-cli -- --dir ./sources --dry-run`
- Rapport JSON : `cargo run -p combinator-cli -- --dir ./sources --report report.json`
- GUI : `cargo run -p combinator-cli --features gui --bin combinator-gui`

## 9. Décisions et références

- ADR : `docs/adr/ADR-0001-workflow-adoption-rust-slint.md`
- PRD : `docs/prd.md` (v0.2.2)
- Architecture : `docs/architecture.md` (v0.2.2)
- Ticket : `docs/tickets/TICKET-000.md`

## 10. Notes de migration / changement de profil

- Ce profil est validé pour la version 0.2.2.
- Toute rupture de stack ou d'outil doit faire l'objet d'un ADR.
- La désactivation par défaut de la GUI doit être préservée sauf décision documentée.