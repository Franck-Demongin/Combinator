# Rapport CI et releases — modèle générique (projet Rust + Slint)

> Document générique, destiné à être adapté à d'autres projets Rust/Slint.
> Les placeholders `<...>` sont à remplacer par les valeurs propres au projet.
> Le projet **Imposer** sert de cas concret en section 7 (Liens et exemple).

Date du rapport : 29 octobre 2026
Version analysée (exemple) : Imposer 0.8.7
Tag publié (exemple) : `v0.8.7`
Branche (exemple) : `master`
État (exemple) : `master` et `v0.8.7` alignés avec `origin`, arbre de travail propre

## 1. Synthèse

Le projet dispose de deux workflows GitHub Actions distincts : une CI multi-plateforme pour valider les changements et un workflow de release déclenché par les tags `v*`. Les workflows sont cohérents, verrouillés avec `--locked`, et couvrent à la fois le binaire CLI et l'interface graphique Slint. Les artefacts finaux sont empaquetés pour Linux et Windows avec checksums SHA-256.

## 2. Workflow CI

Fichier : `.github/workflows/ci.yml`

### Déclenchement
- `push`
- `pull_request`
- Exécution sur `ubuntu-latest` et `windows-latest` sans interruption anticipée (`fail-fast: false`).

### Étapes principales
1. Installation de Rust stable avec `rustfmt` et `clippy`.
2. Installation des dépendances système Linux nécessaires à Slint.
3. Vérification des fichiers et assets requis.
4. Vérification de la cohérence APP_ID et du packaging Linux.
5. `cargo fmt --check`
6. `cargo test --locked`
7. Build CLI sans fonctionnalité GUI : `cargo build --locked`
8. Build avec GUI : `cargo build --features gui --locked` (ou `slint-ui` selon le projet)
9. Clippy strict : `cargo clippy --all-targets --all-features --locked -- -D warnings`

### Contrôles des assets et du packaging
La CI vérifie notamment :
- la présence de `Cargo.lock` dans Git ;
- les PNG d'icônes dans les tailles requises (ex. `16`, `32`, `48`, `64`, `128`, `256`, `512`) ;
- les fichiers source d'icônes et assets d'interface ;
- les scripts Linux d'installation et désinstallation ;
- la licence `LICENSE`.

### Contrôles XDG / GNOME (package Linux)
La CI impose :
- une seule déclaration module-level de `APP_ID` dans `src/slint_ui.rs` (ou fichier équivalent) ;
- aucun identifiant `<app-id-prefix>.` divergent dans `src`, `packaging` et `ui` ;
- `AppWindow::new()` avant `slint::set_xdg_app_id(APP_ID)` ;
- gestion explicite de l'erreur de `set_xdg_app_id(APP_ID)` ;
- un seul fichier `.desktop`, nommé `${APP_ID}.desktop` ;
- les lignes attendues : `Icon=...`, `StartupWMClass=...`, `Exec=<project>-gui`, `Terminal=false` ;
- la validation Freedesktop par `desktop-file-validate`.

### Qualité de la CI
- Mode verrouillé avec `--locked`.
- Vérification simultanée des binaires CLI et GUI.
- Stricte conformité du packaging Linux/XDG.
- Clippy bloquant via `-D warnings`.

## 3. Workflow de release

Fichier : `.github/workflows/release.yml`

### Déclenchement
Tout tag commençant par `v*` :

```yaml
on:
  push:
    tags:
      - "v*"
```

### Permissions
- Job `build` : `contents: read`
- Job `release` : `contents: write`

Seul le job de publication a le droit d'écrire dans le dépôt.

### Job de build
Matrice Linux / Windows, avec :
- build CLI : `cargo build --release --locked --bin <project>`
- build GUI : `cargo build --release --locked --features slint-ui --bin <project>-gui`

### Artifacts Linux
Archive :
- `<project>-<version>-linux-x86_64.tar.gz`

Contenu :
- `<project>`
- `<project>-gui`
- `README.md`
- `LICENSE`
- `<app-id>.desktop`

Un fichier `.sha256` est généré pour l'archive.

### Artifacts Windows
Archive :
- `<project>-<version>-windows-x86_64.zip`

Contenu :
- `<project>.exe`
- `<project>-gui.exe`
- `README.md`
- `LICENSE`

Un fichier `.sha256` est généré pour l'archive.

### Job de publication
- checkout avec historique complet ;
- génération des notes depuis `git log` ;
- téléchargement des artefacts ;
- assemblage dans `release/` ;
- calcul du checksum combiné `SHA256SUMS` ;
- publication avec `softprops/action-gh-release@v2`.

### Artefacts attendus
- `<project>-<version>-linux-x86_64.tar.gz`
- `<project>-<version>-linux-x86_64.tar.gz.sha256`
- `<project>-<version>-windows-x86_64.zip`
- `<project>-<version>-windows-x86_64.zip.sha256`
- `SHA256SUMS`

## 4. Gouvernance des versions

Les releases suivent la convention suivante :
1. développement sur la branche principale (ex. `master` ou `main`)
2. commit de bump de version (`Cargo.toml`, `Cargo.lock`, `QWEN.md` si applicable)
3. création d'un tag annoté `vX.Y.Z`
4. push de la branche et du tag
5. déclenchement automatique du workflow Release

Exemple de gabarit :
- tag : `vX.Y.Z`
- commit : `<hash>`
- message du tag : `<Project> vX.Y.Z`

## 5. Risques et recommandations

1. Le workflow Release dépend entièrement des tags commençant par `v*` ; tout tag erroné peut créer une release involontaire.
2. Toute release requiert un `Cargo.lock` cohérent et committé, sinon `--locked` échoue.
3. Les builds sont limités à `x86_64` ; aucun runner ARM ou autre architecture n'est prévu.
4. Les notes de release dépendent de la qualité des messages de commit.
5. Les assets, le `.desktop` et l'APP_ID sont fortement couplés ; toute modification d'identité requiert une mise à jour synchronisée.
6. La release publie deux binaires, CLI et GUI ; la fonctionnalité GUI reste optionnelle et doit être explicitement activée à la compilation.

## 6. Conclusion

La CI d'un projet Rust/Slint de ce type est stricte, multi-plateforme et centrée sur la stabilité du packaging Linux. La release est entièrement automatisée à partir des tags `v*`, avec production d'archives Linux et Windows et checksums de vérification.

## 7. Liens et exemple concret : Imposer

Le projet **Imposer** (outil Rust d'imposition PDF/PNG/JPEG/PSD) constitue le cas réel à l'origine de ce document. Valeurs spécifiques observées pour Imposer :
- version : `0.8.7`, tag : `v0.8.7`, commit : `e737050`
- message du tag : `Imposer v0.8.7`
- branche : `master` (convention projet ; les futurs dépôts utilisant `main` adapteront)
- identifiant XDG : `io.github.Franck-Demongin.Imposer`
- binaires : `imposer` (CLI) et `imposer-gui` (GUI Slint)
- archive Linux : `imposer-0.8.7-linux-x86_64.tar.gz`, incluant le `.desktop` correspondant
- archive Windows : `imposer-0.8.7-windows-x86_64.zip`
- identité auteur/GitHub : Franck Demongin (`Franck-Demongin`)
- dépôt : `https://github.com/Franck-Demongin/Imposer`

Cette section conserve la trace du projet source tout en rendant le reste du document réutilisable pour d'autres dépôts Rust/Slint.

## 8. Adaptation à un nouveau projet

Points à mettre à jour lors du portage :
- remplacer `<project>` par le nom du crate/binaire ;
- adapter le préfixe APP_ID (`io.github.<owner>.<Project>`) et tous ses usages dans `src`, `packaging`, `ui` ;
- ajuster le nom de la fonctionnalité GUI (`gui` vs `slint-ui`) selon le `Cargo.toml` ;
- vérifier les noms des binaires (`--bin <project>` / `--bin <project>-gui`) ;
- mettre à jour `Exec=` dans le `.desktop` ;
- adapter les tailles d'icônes listées si le projet en utilise d'autres.