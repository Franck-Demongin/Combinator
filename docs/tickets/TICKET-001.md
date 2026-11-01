---
id: TICKET-001
title: Retrait du préfixe verbatim Windows du chemin de sortie GUI
status: done
owner: developer
depends_on: []
created: 2026-11-01
updated: 2026-11-01
---

## Contexte

Le champ Sortie de `combinator-gui`, sous Windows, affichait un préfixe verbatim
`\\?\D:\...` (par exemple `\\?\D:\Users\...\combinator.pdf`) après une génération
réussie. Le fichier était bien généré et ouvrable, mais l'affichage était incorrect.

Cause : `std::fs::canonicalize` renvoie des chemins verbatim (`\\?\C:\...`, voire
`\\?\UNC\...`), affichés tels quels dans l'interface.

Décision : conserver la canonicalisation pour une résolution absolue sûre, puis retirer
le préfixe verbatim avant affichage et avant invocation de l'ouverture externe.

## Critères d'acceptation

- [x] Le champ Sortie affiche un chemin Windows normal (`D:\...\combinator.pdf`) après
  génération.
- [x] Les chemins réseau UNC verbatim sont normalisés (`\\server\share\...`).
- [x] L'ouverture du fichier de sortie et de son dossier reste fonctionnelle.
- [x] Aucun chemin d'action externe ne reçoit le préfixe verbatim restant.

## Tâches

- [x] Identifier la source du préfixe dans `schedule_generation_result`.
- [x] Ajouter `absolute_output_path` pour canonicaliser puis normaliser.
- [x] Ajouter `strip_windows_verbatim_prefix` pour les chemins locaux et UNC.
- [x] Replacer les appels `std::fs::canonicalize` de `output_folder`.
- [x] Normaliser le chemin dans `open_path`.
- [x] Ajouter des tests unitaires du retrait du préfixe.

## Validation (stricte)

- tests : `cargo test --workspace` (vert)
- lint : `cargo clippy --workspace --all-targets --all-features -- -D warnings` (vert)
- build : `cargo build -p combinator-cli` (vert)
- Vérification supplémentaire : `cargo fmt --check` (vert),
  `cargo build -p combinator-cli --features gui` (vert),
  `cargo test -p combinator-cli --features slint-ui --lib` (8 tests verts, dont
  `verbatim_path_tests`)

## Contrôles documentaires (avant `done`)

- [x] Versions et dates cohérentes entre PRD, architecture et ADR touchés (aucun changement
  de version requis pour ce correctif mineur)
- [x] Statut de `docs/stack-profile.md` à jour (`validé`)
- [x] Clauses de phase obsolètes retirées (aucune)

## Notes / liens

- Commit : `3d35baf` — `fix(gui): display Windows output path without verbatim prefix`.
- Fichier modifié : `crates/cli/src/gui.rs`.
- Fonctions : `absolute_output_path`, `strip_windows_verbatim_prefix`.
- Tests : module `verbatim_path_tests`.
- Contexte utilisateur : exécutable Windows fonctionnel, défaut visuel uniquement.