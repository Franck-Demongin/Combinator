# ADR-0001 — Adoption de Workflow et du stack Rust + Slint pour Combinator

- Statut : accepté
- Date : 2026-11-01

## Contexte

Combinator est un projet fonctionnel (crate `combinator-core` + CLI et GUI Slint optionnelle)
sans documentation conforme au cadre Workflow. Les sources, les tests et le pipeline de
release existent déjà, mais aucun PRD, architecture, profil technique ni ticket Workflow
n'était présent. L'opérateur souhaite mettre le projet en conformité documentaire sans
modifier le code source.

Le projet est écrit en Rust (edition 2024, outil `cargo`) et propose une interface graphique
Slint activée par la feature `gui`. Il dépend de Poppler (`pdftoppm`) à l'exécution.

## Décision

- Adopter le cadre documentaire Workflow pour Combinator : `docs/prd.md`, `docs/architecture.md`,
  `docs/stack-profile.md`, `docs/adr/ADR-0001-workflow-adoption-rust-slint.md`,
  `docs/tickets/TICKET-000.md` et mise à jour de `docs/status.md`.
- Documenter uniquement l'existant ; ne pas créer de tickets rétroactifs au-delà de
  TICKET-000 (bootstrap).
- Conserver le stack Rust (`cargo`) avec `rustfmt`, `clippy`, `cargo test`, et Slint pour
  la GUI (feature Cargo `gui`/`slint-ui`).
- Valider les commandes de validation strictes avant de marquer le profil comme validé.
- Ne modifier aucun fichier source ni dépôt externe ; les documents existants
  (`gui-reference.md`, `toast-integration.md`, `slint-rust-gui-reference.md`,
  `imposer_ci_releases_report.md`) sont conservés.

## Conséquences

- Impact positif : référence documentaire alignée sur le code réel ; onboarding et validation
  standardisés ; profil technique validé.
- Impact négatif / coût : maintenance de la documentation à mettre à jour lors des futures
  releases (incréments de version).
- Évolutions attendues : ajout de tickets `TICKET-NNN` pour les évolutions futures ; un ADR
  supplémentaire si le stack change.
- Références : PRD v0.2.2, architecture v0.2.2, `docs/stack-profile.md`,
  `docs/tickets/TICKET-000.md`, `README.md`, `Cargo.toml`.