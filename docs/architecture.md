# Architecture — Combinator

- Version : 0.2.2
- Date : 2026-11-01
- Statut : implémenté
- Racine du projet : `/home/franck/Dev/Rust/combinator`

## 1. Vue d'ensemble

Combinator est un workspace Rust composé de deux crates :

- `combinator-core` : cœur métier indépendant de l'interface (composition A4, lecture des
  sources, erreurs).
- `combinator-cli` : interface en ligne de commande et, sous feature Cargo, interface
  graphique Slint.

Le cœur ne dépend pas de Slint et reste réutilisable pour la CLI. La CLI orchestre la
lecture des sources, la composition, la génération du PDF et l'écriture optionnelle du rapport.
La GUI réutilise les mêmes fonctions de collecte et de génération via `run`.

## 2. Organisation du workspace

```text
.
├── Cargo.toml                        # workspace, version 0.2.2, dépendances communes
├── crates/
│   ├── core/
│   │   ├── Cargo.toml                # combinator-core
│   │   └── src/
│   │       ├── lib.rs                # constantes A4, slot, réexports
│   │       ├── composition.rs        # modèles de composition + compose()
│   │       ├── sources.rs            # extensions, scan, dimensions, ZIP, HEIC
│   │       └── error.rs              # SourceError
│   └── cli/
│       ├── Cargo.toml                # combinator-cli, features gui/slint-ui
│       ├── build.rs                  # (icône Windows sous cfg windows)
│       ├── src/
│       │   ├── main.rs               # binaire combinator -> run()
│       │   ├── lib.rs                # run, gui (feature gui)
│       │   ├── run.rs                # CLI, rasterisation, PDF, rapport
│       │   ├── gui.rs                # interface Slint (feature gui)
│       │   └── gui_main.rs           # binaire combinator-gui
│       ├── ui/                       # définitions Slint (.slint)
│       └── tests/
│           ├── cli.rs                # tests d'intégration CLI
│           └── placement.rs           # test de slot
├── packaging/linux/                   # installation locale (.desktop, icônes, scripts)
├── assets/                            # icônes applicatives
└── docs/                              # documentation Workflow
```

## 3. Mapping des modules

### 3.1 `combinator-core`

| Fichier | Rôle principal | Éléments publics clés |
|---|---|---|
| `lib.rs` | constantes géométriques et dérivés | `A4_WIDTH_MM`, `A4_HEIGHT_MM`, `PRINT_MARGIN_MM`, `PAIRS_PER_BOARD`, `SOURCES_PER_BOARD`, `SLOT_GAP_MM`, `BoardRegion`, `source_slot_dimensions()` |
| `composition.rs` | modèles sérialisables et algorithme de placement | `SourceDimensions`, `Rotation`, `FitStatus`, `Placement`, `Board`, `SourceIssue`, `CompositionReport`, `compose()` |
| `sources.rs` | formats supportés, scan, extraction de dimensions, ZIP | `supported_extensions()`, `is_supported_path()`, `expand_directory()`, `expand_zip()`, `dimensions_from_path()` |
| `error.rs` | erreurs de lecture de sources | `SourceError` (Io, UnsupportedFormat, Image, Pdf, Heic, InvalidDimensions, Zip, ReadDirectory) |

### 3.2 `combinator-cli`

| Fichier | Rôle principal |
|---|---|
| `main.rs` | point d'entrée du binaire `combinator`, appelle `run()` |
| `lib.rs` | expose `run` et, sous feature `gui`, le module `gui` |
| `run.rs` | parsing CLI (`clap`), découverte, rasterisation, composition, écriture PDF, rapport JSON, dry-run, `collect_source_files`, `generate_collected` |
| `gui.rs` | application Slint : état, actions, génération asynchrone, toasts, rapport |
| `gui_main.rs` | point d'entrée du binaire `combinator-gui` (feature `slint-ui`) |

## 4. Flux de données

### 4.1 Découverte de sources (CLI)

1. `run()` (run.rs) parse les arguments (`Cli` derive `Parser`).
2. Construction de `input_paths` à partir de `--source`, `--dir` et des arguments positionnels.
3. Pour chaque chemin :
   - répertoire : `expand_directory()` (top level) ;
   - fichier : extraction ZIP (`expand_zip`) ou ajout direct si format pris en charge.
4. Tri naturel (`natord::compare`).
5. Erreur si aucune source.
6. Validation des destinations (`validate_output`) avant rasterisation si non dry-run.

`collect_source_files()` factorise la logique de découverte réutilisée par la GUI.

### 4.2 Découverte de sources (GUI)

La GUI construit un vecteur de chemins (`source_list`) puis appelle `collect_source_files()`
pendant la génération. Les entrées ZIP sont extraites dans le répertoire temporaire OS.

### 4.3 Extraction des dimensions

`dimensions_from_path()` route selon l'extension :

- images (`png`, `jpg`, `jpeg`, `webp`) : `image::ImageReader`, DPI déduit du fichier ;
- `pdf` : `lopdf` lit les pages et `MediaBox` (avec héritage parent), `UserUnit`, `Rotate` ;
- `heic`/`heif` : `heic_rs::probe`, DPI par défaut 300 ;
- autres : `SourceError::UnsupportedFormat`.

Chaque source produit une ou plusieurs `SourceDimensions` (une par page pour les PDF).

### 4.4 Rasterisation

Pour l'écriture du PDF, chaque source est rasterisée :

- images et WebP : décodage `image`, conversion RGB avec composition alpha sur blanc ;
- HEIC/HEIF : décodage `heic-rs`, conversion par `build_heic_image` ;
- PDF : invocation de `pdftoppm -png -r 300` dans un répertoire temporaire,
  lecture des PNG générés ;
- DPI final : valeur mesurée pour les images, 300 DPI pour HEIC et PDF rasterisés.

### 4.5 Composition A4

`compose()` (composition.rs) :

1. Calcule le slot via `source_slot_dimensions()` (5 mm marge, gap 5 mm, 2 slots).
2. Filtre les sources invalides en les consignant dans `issues` + `warnings`.
3. Pour chaque paire (chunks de 2), calcule pour chaque source l'orientation et l'échelle
   qui maximisent le remplissage du slot.
4. Construit `Board` et `Placement`.
5. Émet des avertissements pour source isolée, sources invalides et liste vide.

### 4.6 Génération du PDF

`write_pdf()` (run.rs) :

1. Crée un `Document` lopdf version 1.7.
2. Pour chaque raster, encode un XObject JPEG qualité 95.
3. Pour chaque board (planche) :
   - construit les opérateurs de contenu pour chaque placement (cm/Do/Q) avec centrage
     et rotation à 90° si nécessaire ;
   - crée une page `Page` avec `MediaBox` A4 et ressources `XObject`.
4. Crée l'objet `Pages` et le `Catalog`, fixe la racine.
5. Crée les répertoires parents puis sauvegarde le fichier.

### 4.7 Rapport

- Textuel : `GenerationSummary.report_text` (lignes synthétiques de placement).
- JSON (`--report`) : enveloppe `version: 1`, `sources`, `composition` ; arrondi numérique à
  2 décimales (`round_report_json`).
- GUI : affichage du rapport textuel dans une boîte de dialogue Slint.

### 4.8 Interface graphique (feature `gui`)

`gui::run()` crée `MainWindow`, installe les toasts, branche les callbacks et lance
l'event loop Slint. La génération est déclenchée dans un thread de fond ; les mises à
jour de progression sont renvoyées via `slint::invoke_from_event_loop`. Le résultat est
affiché à la fin, avec toast et rapport.

## 5. Caractéristiques de build

### 5.1 Features Cargo (`combinator-cli`)

- `default = []`.
- `gui = ["dep:slint", "dep:rfd", "dep:slint-build"]`.
- `slint-ui = ["gui"]` (alias utilisé par le binaire GUI).

Binaires :

- `combinator` : `src/main.rs`, sans dépendance feature.
- `combinator-gui` : `src/gui_main.rs`, `required-features = ["gui"]`.

### 5.2 Build Slint

- `slint-build` est une dépendance de build optionnelle.
- L'interface est embarquée via `slint::include_modules!()`.

### 5.3 Compilation Windows

- Dépendance de build `winresource` sous `cfg(windows)` pour l'icône d'application.

## 6. Dépendances système et runtime

- Rust 1.99 (edition 2024).
- Poppler `pdftoppm` pour PDF source.
- GUI Linux : libfontconfig, freetype, x11, xkbcommon, wayland, desktop-file-utils.
- Portail de fichiers : `rfd` avec `xdg-portal`.

## 7. Tests et validation

- Tests unitaires (core) : composition et slot.
- Tests d'intégration CLI (`crates/cli/tests/cli.rs`) : aide, scan niveau unique,
  multi-page PDF, rapport JSON par page, WebP, écrasement forcé.
- Test placement GUI (`crates/cli/tests/placement.rs`).
- Validation stricte du projet (voir profil technique) : fmt, test, clippy, build CLI et
  CLI avec feature gui.

## 8. Installation locale

- Script `packaging/linux/install-local.sh` :
  - copie le binaire vers `~/.local/bin` ;
  - installe `.desktop` et icônes hicolor ;
  - met à jour les caches.
- Identité : `io.github.Franck-Demongin.Combinator`.
- Désinstallation via `packaging/linux/uninstall-local.sh`.

## 9. Limites actuelles

- Scan de répertoire non récursif.
- PDF chiffré non supporté.
- Aucun rapport JSON en sortie GUI.
- Extraction ZIP temporaire, nettoyage implicite par l'OS.
- HEIC supposé 300 DPI.
- Lint clippy configuré en avertissement (`all`, `pedantic`) mais la validation en
  échec utilise `-D warnings`.

## 10. Références

- `docs/prd.md`, `docs/stack-profile.md`, `docs/adr/ADR-0001-workflow-adoption-rust-slint.md`
- `docs/gui-reference.md`, `docs/slint-rust-gui-reference.md`, `docs/toast-integration.md`
- `README.md`
- `Cargo.toml`, `crates/core/Cargo.toml`, `crates/cli/Cargo.toml`