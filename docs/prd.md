# PRD — Combinator

- Version : 0.2.2
- Date : 2026-11-01
- Statut : implémenté (release `v0.2.2`)
- Racine du projet : `/home/franck/Dev/Rust/combinator`

## 1. Vision et objectif

Combinator compose automatiquement des planches PDF au format A4 à partir de sources
graphiques et documentaires. Il prend des images (PNG, JPEG, WebP, HEIC/HEIF), des pages
PDF et des archives ZIP, calcule un placement optimal dans des slots A4 fixes, rasterise
et écrit un PDF composé. Une interface graphique Slint (optionnelle, feature Cargo `gui`)
expose le même pipeline avec sélection native des sources et retour utilisateur.

L'objectif est de produire des planches PDF imprimables sans intervention manuelle pour
l'agencement, tout en conservant un rapport détaillé de la composition.

## 2. Périmètre

### 2.1 Dans le périmètre

- Composition de planches A4 (297 × 210 mm) à deux sources par planche.
- Formats source : PNG, JPEG, WebP, PDF (chaque page devient une source), HEIC/HEIF.
- Extraction de sources depuis une archive ZIP (entrées prises en charge, y compris
  sous-dossiers de l'archive).
- Scan d'un répertoire source au niveau supérieur uniquement (pas de récursion dans les
  sous-dossiers du disque).
- Tri naturel des chemins source via `natord`.
- Sortie PDF principal au chemin choisi par l'utilisateur (`combinator.pdf` par défaut).
- Rapport JSON optionnel contenant la liste des sources et la composition détaillée.
- Mode `dry-run` : affiche les sources lues et la sortie prévue sans écrire le PDF.
- Paramètre `--force` pour écraser une sortie existante (rapport compris).
- Interface graphique Slint (feature `gui`) : sélection de fichiers, de dossier, d'archive
  ZIP, suppression, réordonnancement par glisser-déposer, chemin de sortie, ouverture du
  fichier/dossier, lancement asynchrone, progression, toasts, visualisation du rapport.
- Installation locale Linux : binaire, entrée de bureau, icônes, caches.
- Version affichée dynamiquement depuis `CARGO_PKG_VERSION` (GUI).

### 2.2 Hors périmètre à ce jour

- Aucune édition ou recadrage interactif des sources.
- Aucune récursion dans les répertoires du disque pour la découverte de sources.
- Aucune gestion des PDF chiffrés.
- Aucun serveur, aucune API réseau, aucune persistance de projet dédiée au-delà des
  fichiers et du fichier `.desktop`/icônes locaux.
- La GUI ne génère pas un rapport JSON sur disque ; le rapport textuel est affiché dans
  la fenêtre.

## 3. Utilisateurs et cas d'usage

- **Utilisateur final (desktop)** : prépare des planches pour impression, utilise la GUI
  pour sélectionner des sources et générer un PDF.
- **Utilisateur avancé / intégration** : utilise le binaire `combinator` en ligne de
  commande, avec sortie silencieuse (`-q`) et rapport JSON pour chaînage.
- **Opérateur de build/release** : compile les binaires et installe localement
  (`cargo build --features gui`, script `packaging/linux/install-local.sh`).

Cas d'usage typiques :

- `combinator --dir ./sources -o report.pdf`
- `combinator --source a.png b.jpg c.pdf -o report.pdf`
- `combinator --dir ./sources --dry-run`
- `combinator --dir ./sources --report report.json`
- Lancement de `combinator-gui`, sélection de sources et génération vers le Bureau par
  défaut.

## 4. Fonctionnalités détaillées

### 4.1 Interface en ligne de commande

Le binaire `combinator` est distribué sous `crates/cli` (pas de GUI par défaut).

Options principales :

| Option | Description |
|---|---|
| `--source`, `-s` | Fichier ou dossier source, répétable |
| `--dir`, `-d` | Répertoire source unique au niveau supérieur |
| arguments positionnels `INPUT` | Fichiers ou répertoires |
| `--output`, `-o` | Chemin du PDF composé (défaut `combinator.pdf`) |
| `--report` | Chemin d'un rapport JSON complémentaire |
| `--force`, `-f` | Écrase une sortie existante |
| `--dry-run` | Affiche le plan sans écrire le PDF |
| `--quiet`, `-q` | Sortie silencieuse en cas de succès |

Règles :

- Au moins une source doit être fournie (sinon erreur).
- Un répertoire est lu au niveau supérieur uniquement ; les sous-dossiers sont ignorés.
- Une archive ZIP valide extrait ses entrées prises en charge (PNG, JPEG, WebP, PDF,
  HEIC/HEIF), y compris celles situées dans des sous-dossiers de l'archive.
- Si aucune source valide n'est trouvée, erreur.
- En cas de `dry-run`, aucune écriture de sortie n'a lieu ; les dimensions rasterisées
  et la sortie prévue sont affichées.
- La sortie principale doit être un PDF (extension `.pdf` ou ajout automatique) ; un chemin
  sans extension est interprété comme fichier PDF.
- Un rapport JSON est validé avant écriture et respecte `--force`.

### 4.2 Découverte et lecture des sources

- L'extension détermine le traitement (`png`, `jpg`, `jpeg`, `webp`, `pdf`, `heic`,
  `heif`, `zip`).
- Images rasterisées via `image` ; DPI déduit du fichier (PNG `pHYs`, JPEG JFIF), sinon
  300 DPI.
- HEIC/HEIF décodés via `heic-rs` ; DPI par défaut 300 (les métadonnées HEIC ne
  fournissent pas toujours un DPI exploitable).
- PDF source : chaque page est rasterisée par `pdftoppm` (Poppler) à 300 DPI puis lue
  comme PNG généré.
- PDF chiffré : non pris en charge (erreur explicite).
- PDF sans page ou sans `MediaBox` : erreur.
- Les dimensions invalides (non finies ou ≤ 0) sont marquées comme problème dans le rapport
  et ignorées pour la composition.

### 4.3 Composition A4

- Page A4 : 297 × 210 mm.
- Zone utile : marge d'impression de 5 mm sur chaque bord.
- Deux slots de source côte à côte, gap de 5 mm entre les slots.
- Calcul par source : orientation directe et rotation de 90° sont comparées, la meilleure
  échelle de remplissage du slot est choisie.
- `FitStatus` : `Fits`, `ScaledUp`, `ScaledDown`.
- Sources traitées par paires ; une source sans paire occupe uniquement la moitié gauche
  de la planche (avertissement).
- Aucun fichier source n'est écrit si le PDF ne peut pas être généré.

### 4.4 Sortie PDF

- Un XObject JPEG (qualité 95) par source.
- Contenu PDF généré avec `lopdf`, boîtes de page au format A4.
- Les placements respectent rotation, centrage et échelle.
- Le PDF est écrit sur le chemin résolu ; les répertoires parents sont créés si nécessaire.
- Le rapport JSON optionnel contient `version: 1`, `sources` (chemin, page, largeur/hauteur mm)
  et `composition` (détails des planches, placements, problèmes, avertissements).

### 4.5 Interface graphique Slint (MVP)

Activée par la feature Cargo `gui` (qui active `slint`, `rfd`, `slint-build`).

Fonctionnalités présentes dans `crates/cli/src/gui.rs` :

- Sélection de fichiers sources via dialogue natif (`png`, `jpg`, `jpeg`, `webp`, `heic`,
  `heif`, `pdf`).
- Ajout d'un répertoire (lecture niveau supérieur via `combinator_core::expand_directory`).
- Ajout d'une archive ZIP via `combinator_core::expand_zip`.
- Liste des sources avec nom tronqué et chemin tronqué, nombre de sources dans le titre
  de section.
- Suppression d'une source et bouton « Vider » désactivé si la liste est vide.
- Réordonnancement par glisser-déposer.
- Chemin de sortie par défaut : `<Bureau>/combinator.pdf`.
- Sélection d'un fichier de sortie PDF.
- Options d'ouverture du fichier généré et de son dossier.
- Case à cocher d'écrasement forcé (équivalent `--force`).
- Progression et message de statut pendant la génération asynchrone (thread de fond).
- Affichage du rapport textuel dans une boîte de dialogue dédiée.
- Toasts de succès/avertissement/erreur.
- Version dynamique dans le titre de fenêtre (`env!("CARGO_PKG_VERSION")`).
- Verrouillage des actions pendant la génération (`busy`).

Limites connues de la GUI :

- Pas de création de rapport JSON sur disque (le rapport est affiché dans l'application).
- Pas de mode `dry-run`.
- Pas d'argument de sortie sans extension détectée au-delà de la résolution interne.

### 4.6 Installation et persistance locale (Linux)

- Binaire GUI : `combinator-gui` (feature `slint-ui`, qui active `gui`).
- Installateur : `packaging/linux/install-local.sh <chemin-du-binaire>`.
  - copie le binaire vers `~/.local/bin` ;
  - installe l'entrée de bureau `io.github.Franck-Demongin.Combinator.desktop` ;
  - installe les icônes hicolor (`16` à `512` px, plus SVG) ;
  - rafraîchit les caches `applications` et icônes.
- Désinstallateur : `packaging/linux/uninstall-local.sh`.
- `~/.local/bin` doit être dans `PATH`.
- Vérification : `desktop-file-validate ...` et `gtk-update-icon-cache -f ...`.
- Les archives de release Linux sont extraite puis installées avec le même script.

### 4.7 Identité application

- Application ID (XDG) : `io.github.Franck-Demongin.Combinator`.
- Binaires : `combinator` et `combinator-gui`.
- Catégories de bureau : `Office;`.

## 5. Dépendances système

### 5.1 Ubuntu / Debian

- Bibliothèques de build Slint/GUI : `libfontconfig1-dev`, `libfreetype6-dev`, `libx11-dev`,
  `libxkbcommon-dev`, `libwayland-dev`, `desktop-file-utils`.
- Poppler : `poppler-utils` fournit `pdftoppm`, utilisé pour rasteriser les pages PDF source.
- Équivalents nécessaires sur Windows avec `pdftoppm` disponible.

### 5.2 Dépendances Rust (workspace)

- Noyau : `thiserror`, `serde`, `image` (png, jpeg, webp), `heic-rs` (std), `lopdf`, `zip`,
  `natord`.
- CLI : `anyhow`, `clap`, `tempfile`, `serde_json`.
- GUI (optionnel) : `slint` (backend winit, rendu femtovg/software, accessibilité,
  compatibilité 1.2), `rfd` (portail XDG + tokio), `slint-build`.
- Workspace : `anyhow`, `thiserror`, `tracing` déclarés au niveau workspace.

## 6. Exigences non fonctionnelles

- Messages utilisateur en français, avec accents corrects.
- Erreurs explicites sur l'entrée (`format non supporté`, `chemin introuvable`, etc.).
- Absence de `unsafe` tolérée avec avertissement (`unsafe_code = "warn"`).
- Validation stricte : `cargo fmt --check`, `cargo test --workspace`,
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`,
  `cargo build -p combinator-cli`, `cargo build -p combinator-cli --features gui`.
- Codes de sortie non nul en cas d'erreur CLI.
- Pipeline déterministe : tri naturel des sources, ordre stable.

## 7. Rapport JSON — forme générale

```json
{
  "version": 1,
  "sources": [
    { "path": "...", "page": 0, "width_mm": 100.0, "height_mm": 50.0 }
  ],
  "composition": {
    "board_width_mm": 297.0,
    "board_height_mm": 210.0,
    "print_margin_mm": 5.0,
    "slot_width_mm": 0.0,
    "slot_height_mm": 0.0,
    "slot_gap_mm": 5.0,
    "source_count": 0,
    "board_count": 0,
    "boards": [],
    "issues": [],
    "warnings": []
  }
}
```

Les valeurs numériques du rapport sont arrondies à deux décimales lors de la sérialisation.

## 8. Limites et risques connus

- Dépendance externe `pdftoppm` (Poppler) : doit être présent dans le `PATH`.
- L'extraction ZIP écrit dans un répertoire temporaire (`std::env::temp_dir`) et ne valide
  pas les liens/chemins absolus au-delà du rejet des composants non normaux
  (`zip_entry_target`).
- Les images HEIC sont supposées à 300 DPI par défaut.
- Le scan de répertoire n'est pas récursif.
- Le PDF source doit être non chiffré.
- La GUI ne produit pas de rapport JSON sur disque.

## 9. Historique des versions

| Version | Tag | Description synthétique |
|---|---|---|
| 0.1.0 | `v0.1.0` | moteur de composition et CLI pour A4 à partir de formats supportés |
| 0.1.1 | `v0.1.1` | GUI : Bureau par défaut et résolution du dossier de sortie |
| 0.2.0 | `v0.2.0` | améliorations GUI (compteur, pourcentage, glisser-déposer) |
| 0.2.1 | `v0.2.1` | version dynamique dans le titre de fenêtre |
| 0.2.2 | `v0.2.2` | support ZIP, corrections icône Windows, release préparée |

## 10. Références

- `README.md`
- `crates/core/src/lib.rs`, `composition.rs`, `sources.rs`, `error.rs`
- `crates/cli/src/run.rs`, `gui.rs`, `gui_main.rs`, `main.rs`, `lib.rs`
- `crates/cli/Cargo.toml`, `Cargo.toml`
- `docs/gui-reference.md`, `docs/slint-rust-gui-reference.md`, `docs/toast-integration.md`
- `docs/imposer_ci_releases_report.md`
- `docs/architecture.md`, `docs/adr/ADR-0001-workflow-adoption-rust-slint.md`,
  `docs/stack-profile.md`, `docs/status.md`, `docs/tickets/TICKET-000.md`