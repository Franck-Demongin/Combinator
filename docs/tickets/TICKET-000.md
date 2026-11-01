---
id: TICKET-000
title: Bootstrap de la documentation Workflow pour l'existant
status: done
owner: analyst-architect
depends_on: []
created: 2026-11-01
updated: 2026-11-01
---

## Contexte

Combinator est un projet fonctionnel à la version 0.2.2 (crate `combinator-core` + CLI et
GUI Slint optionnelle) sans documentation conforme au cadre Workflow. Ce ticket couvre la mise
en conformité documentaire sans modification du code source.

Livrables créés/mis à jour :

- `docs/prd.md` (v0.2.2)
- `docs/architecture.md` (v0.2.2)
- `docs/adr/ADR-0001-workflow-adoption-rust-slint.md`
- `docs/stack-profile.md` (validé pour la version 0.2.2)
- `docs/status.md`
- `docs/tickets/TICKET-000.md`

Les documents existants (`gui-reference.md`, `toast-integration.md`,
`slint-rust-gui-reference.md`, `imposer_ci_releases_report.md`) sont conservés.

## Critères d'acceptation

- [x] PRD décrit l'application réelle (sources PNG/JPEG/WebP/PDF/HEIC/HEIF/ZIP,
  rapport JSON, dry-run, CLI + GUI Slint MVP, installation locale,
  dépendances système dont Poppler).
- [x] Architecture décrit le workspace, le module mapping et les flux.
- [x] ADR explique l'adoption documentaire et le stack Rust.
- [x] Profil technique renseigné depuis le template avec commandes de validation stricte.
- [x] Aucun fichier source modifié.
- [x] Commandes de validation strictes exécutées avec succès.

## Tâches

- [x] Lire les sources, le README et l'historique Git.
- [x] Rédiger `docs/prd.md`.
- [x] Rédiger `docs/architecture.md`.
- [x] Rédiger l'ADR.
- [x] Rédiger `docs/stack-profile.md` et le valider.
- [x] Rédiger `docs/tickets/TICKET-000.md`.
- [x] Mettre à jour `docs/status.md`.
- [x] Exécuter les commandes de validation strictes.

## Validation (stricte)

- tests : `cargo test --workspace`
- lint : `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- build : `cargo build -p combinator-cli`
- Vérification supplémentaire : `cargo fmt --check` et `cargo build -p combinator-cli --features gui`

## Contrôles documentaires (avant `done`)

- [x] Versions et dates cohérentes entre PRD, architecture et ADR touchés (v0.2.2, 2026-11-01)
- [x] Statut de `docs/stack-profile.md` à jour (`validé`)
- [x] Clauses de phase obsolètes retirées

## Notes / liens

- Commandes validées le 2026-11-01 : `cargo fmt --check`, `cargo test --workspace`,
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`,
  `cargo build -p combinator-cli`, `cargo build -p combinator-cli --features gui` (toutes vertes).
- Ticket de bootstrap uniquement ; pas de ticket rétroactif pour l'historique complet.