# Guide de référence : développement Rust + Slint (application de bureau)

Document interne consolidé pour réutiliser la stratégie retenue dans Imposer pour l'identité applicative, les icônes, le packaging Linux/GNOME/Wayland, les binaires Windows, le thème clair/sombre et l'interface Slint. Objectif : éviter les tâtonnements et servir de base de travail pour un futur projet Rust + Slint.

Ce document est volontairement exhaustif et ancré dans le dépôt Imposer (état actuel : `Cargo.toml` 0.8.1, source Slint dans `ui/main.slint`, interface Rust dans `src/slint_ui.rs`).

---

## 1. Vue d'ensemble de l'approche

- Rust 2024, GUI Slint optionnelle et second binaire : `imposer-gui`.
- GUI gated derrière la feature Cargo `slint-ui` (alias `gui`).
- Build par défaut : uniquement `imposer` (CLI + TUI ratatui).
- Build avec GUI : `cargo build --features gui --bin imposer-gui`.
- Le code Slint est compilé au moment de la compilation par `slint_build::compile("ui/main.slint")` dans `build.rs`.
- Le front Slint et le moteur métier partagent la bibliothèque `imposer` (calcul de mise en page, génération, configuration, presets).
- L'UI ne bloquant jamais le thread d'interface : la génération tourne via `run_imposition_work` et un canal `mpsc` interrogé par un `slint::Timer` de 80 ms.

Fichiers structurants :

- `Cargo.toml` : dépendances optionnelles, features, binaires.
- `src/main.rs` : CLI/TUI (binaire par défaut).
- `src/gui_main.rs` : point d'entrée GUI, `windows_subsystem`, définition locale du module `slint_ui`.
- `src/slint_ui.rs` : logique GUI, callbacks, état Slint, constantes `APP_ID`.
- `ui/main.slint` : composant `AppWindow`, tokens `Theme`, palette système.
- `build.rs` : compilation Slint et injection des métadonnées/icône Windows.
- `assets/icons/` : SVG source, PNG dérivés, ICO Windows, icônes UI embarquées.
- `packaging/linux/` : `.desktop`, scripts d'installation/désinstallation.
- `.github/workflows/ci.yml` et `release.yml` : validation et publication.

---

## 2. Architecture Rust + Slint

### 2.1. Feature gating

Ne pas mettre Slint dans les dépendances non optionnelles. Pattern retenu :

```toml
[features]
default = []
gui = ["slint-ui"]
slint-ui = ["dep:slint", "dep:rfd", "dep:slint-build"]

[dependencies]
slint = { version = "1", default-features = false, features = [
    "std",
    "compat-1-2",
    "accessibility",
    "backend-winit",
    "renderer-femtovg",
    "renderer-software",
], optional = true }
rfd = { version = "0.15", default-features = false, features = ["xdg-portal", "tokio"], optional = true }

[build-dependencies]
slint-build = { version = "1", optional = true }

[target.'cfg(windows)'.build-dependencies]
winresource = "0.1"

[[bin]]
name = "imposer"
path = "src/main.rs"

[[bin]]
name = "imposer-gui"
path = "src/gui_main.rs"
required-features = ["slint-ui"]
```

Points importants :

- `default-features = false` pour Slint : on déclare explicitement `backend-winit`, `renderer-femtovg` et `renderer-software`. Cela évite de tirer des styles/backends non souhaités.
- `compat-1-2` et `accessibility` sont activés pour stabilité et a11y.
- `rfd` en mode `xdg-portal` : sur Linux, les dialogues de fichiers passent par le portail XDG et demandent la feature `tokio`.
- `winresource` est une dépendance de build cible Windows uniquement.
- `gui` est un alias pratique ; le nom canonique interne est `slint-ui`.

### 2.2. Point d'entrée GUI

`src/gui_main.rs` :

```rust
#![cfg_attr(all(windows, feature = "slint-ui"), windows_subsystem = "windows")]

#[cfg(feature = "slint-ui")]
#[allow(dead_code)]
mod tui;

#[cfg(feature = "slint-ui")]
mod slint_ui;

#[cfg(feature = "slint-ui")]
fn main() {
    if let Err(e) = slint_ui::run() {
        eprintln!("erreur Slint : {e}");
        std::process::exit(1);
    }
}
```

Notes :

- `windows_subsystem = "windows"` évite la console CMD lors du lancement GUI.
- Le module `tui` est déclaré mais `#[allow(dead_code)]` car il peut ne pas être utilisé en mode GUI pur.
- Tout code spécifique à Windows (`winresource`) reste hors du code Rust applicatif grâce à `build.rs`.

### 2.3. Build script

`build.rs` distingue trois branches :

```rust
#[cfg(all(feature = "slint-ui", target_os = "windows"))]
fn main() {
    let mut res = winresource::WindowsResource::new();
    res.set_icon("assets/icons/imposer.ico");
    res.set("FileDescription", "Imposition de documents PDF, PNG, JPEG et PSD");
    res.set("ProductName", "Imposer");
    res.set("OriginalFilename", "imposer-gui.exe");
    res.compile().expect("échec de la compilation des ressources Windows");
    #[cfg(feature = "slint-ui")]
    slint_build::compile("ui/main.slint").expect("échec de la compilation Slint");
}

#[cfg(all(feature = "slint-ui", not(target_os = "windows")))]
fn main() {
    slint_build::compile("ui/main.slint").expect("échec de la compilation Slint");
}

#[cfg(not(feature = "slint-ui"))]
fn main() {
}
```

Pièges :

- **Correction importante** : dans un `build.rs`, `#[cfg(target_os = "...")]` reflète l'HÔTE (machine qui compile), pas la cible. La cible se lit via la variable `CARGO_CFG_TARGET_OS` (`std::env::var("CARGO_CFG_TARGET_OS")`). Le code actuel reste valide car les builds Windows du projet sont natifs sur `windows-latest` ; une cross-compilation (voir §7.3) ne serait pas fiable avec ce test direct.
- Si la feature n'est pas activée, `build.rs` ne fait rien ; ainsi les builds par défaut restent légers.
- L'échec d'une commande `res.compile()` doit rester explicite (`.expect`) pour éviter un binaire sans icône silencieux.

### 2.4. Cycle de vie Slint et génération hors thread UI

Dans `src/slint_ui.rs`, le pattern est :

1. création de `AppWindow::new()`;
2. `slint::set_xdg_app_id(APP_ID)` juste après `AppWindow::new()` et avant la boucle d'événements (ordre vérifié par test réel);
3. mise à jour des propriétés initiales (`set_sheet_options`, dimensions, etc.);
4. enregistrement des callbacks (`on_browse_source_one`, `on_generate`, etc.);
5. timer 80 ms qui lit un `Arc<Mutex<Option<BusyReceiver>>>` et récupère `(status, Option<PathBuf>)`;
6. `ui.run()` implicite en fin de `run()` pour entrer dans la boucle d'événements.

Bonnes pratiques confirmées par le projet :

- Toujours passer par `weak_ui.clone()` dans les closures plutôt que de conserver `ui` directement, pour éviter les références circulaires et les borrow checkers.
- Vérifier `ui.get_busy()` au début des callbacks déclenchés par l'utilisateur pour ignorer les clics concurrents.
- Mettre à jour les propriétés Slint avec les setters générés (`set_status`, `set_sheet_index`, etc.).
- Appeler `invoke_refresh_preview()` plutôt que d'exécuter toute la logique synchrone dans une closure si cela met à jour l'état.
- Conserver les `Timer` Slint dans une `Rc<RefCell<Vec<Timer>>>` pour éviter qu'ils soient détruits prématurément.

---

## 3. Identité applicative : APP_ID

### 3.1. Définition

```rust
// src/slint_ui.rs
pub const APP_ID: &str = "io.github.Franck-Demongin.Imposer";

pub fn run() -> Result<()> {
    let ui = AppWindow::new().map_err(|e| ImposerError::Generation(e.to_string()))?;

    // IMPORTANT : appeler set_xdg_app_id APRÈS AppWindow::new().
    // Constaté sous GNOME/Wayland : appelée avant, l'icône n'est pas appliquée.

    if let Err(e) = slint::set_xdg_app_id(APP_ID) {
        eprintln!("avertissement : app_id non appliqué : {e}");
    }
    ...
}
```

Règles strictes :

- Utiliser un identifiant inverse de domaine (`io.github.<nom-utilisateur>.<Application>`).
- Appeler `slint::set_xdg_app_id` tôt, avant la boucle d'événements, idéalement juste après `AppWindow::new()`.
- Ne pas créer deux versions de l'identifiant (`Imposer`, `imposer`, etc.). Une seule source de vérité.
- Le même identifiant doit être repris dans :
  - `packaging/linux/io.github.Franck-Demongin.Imposer.desktop`;
  - `StartupWMClass=io.github.Franck-Demongin.Imposer`;
  - `Icon=io.github.Franck-Demongin.Imposer`;
  - noms de fichiers icônes hicolor (`hicolor/<taille>x<taille>/apps/io.github.Franck-Demongin.Imposer.png`);
  - script d'installation `APP_ID`.

### 3.2. Pourquoi c'est critique sous GNOME/Wayland

GNOME associe un fichier `.desktop` à une fenêtre via le nom de fichier du lanceur et `StartupWMClass`. Si le fichier s'appelle `imposer.desktop` alors que l'application annonce `io.github.Franck-Demongin.Imposer`, l'icône dans le tableau de bord, l'association automatique de fenêtre et l'épinglage peuvent être incorrects. Ne jamais renommer le fichier `.desktop` sans changer l'APP_ID et reconstruire le binaire.

Piège constaté : `set_xdg_app_id` avant `AppWindow::new()` => icône non appliquée sous GNOME/Wayland.

### 3.3. Check-list APP_ID

- Le nom du fichier `.desktop` est exactement `<APP_ID>.desktop`.
- `Icon=` contient `<APP_ID>` sans extension.
- `StartupWMClass=` contient `<APP_ID>`.
- Les icônes hicolor installées sont nommées `<APP_ID>.{png,svg}`.
- `APP_ID` est déclaré au niveau du module (`pub const APP_ID`), sur une seule ligne, jamais dans une fonction.
- `set_xdg_app_id(APP_ID)` est appelé après `AppWindow::new()` et son résultat est testé (`if let Err(e)`).
- Ces règles sont protégées par les contrôles CI correspondants de `.github/workflows/ci.yml`.
- Le script d'uninstall supprime strictement les mêmes chemins.

---

## 4. Build et outillage

### 4.1. Commandes de développement

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build
cargo build --features gui
cargo build --release --features gui --bin imposer-gui
cargo build --release --bin imposer
```

### 4.2. Dépendances système sous Ubuntu

Le job CI installe :

```sh
sudo apt-get install -y \
  libfontconfig1-dev \
  libfreetype6-dev \
  libx11-dev \
  libxkbcommon-dev \
  libwayland-dev
```

Notes :

- Slint avec backend winit a besoin de ces paquets pour compiler et tourner sous X11/Wayland.
- Sans eux, les erreurs apparaissent lors de la résolution des crates natives, pas dans le code Rust.
- Conserver ces paquets dans toute procédure d'installation ou Docker de développement.

### 4.3. Conventions de versioning

- `version` dans `Cargo.toml` est la source de vérité.
- Le titre de fenêtre GUI utilise `env!("CARGO_PKG_VERSION")` : `"Imposer - v{version}"`.
- Les tags sont annotés `v<majeur>.<mineur>.<patch>`.
- Conventional Commits en anglais pour les messages (`feat:`, `fix:`, `ci:`, `chore:`, etc.).

---

## 5. Icônes : source SVG, rasterisation, intégration

### 5.1. Source et raster dérivés

Source vectorielle :

```
assets/icons/imposer.svg
```

Sorties attendues et vérifiées par CI :

```
assets/icons/imposer-16.png
assets/icons/imposer-32.png
assets/icons/imposer-48.png
assets/icons/imposer-64.png
assets/icons/imposer-128.png
assets/icons/imposer-256.png
assets/icons/imposer-512.png
assets/icons/imposer.ico
assets/icons/imposer.svg
```

Le script de régénération est :

```sh
./assets/icons/generate.sh
```

Son contenu :

```sh
SOURCE="assets/icons/imposer.svg"
OUTPUT_DIR="assets/icons"

mkdir -p "$OUTPUT_DIR"
for SIZE in 16 32 48 64 128 256 512; do
  rsvg-convert -w "$SIZE" -h "$SIZE" "$SOURCE" -o "$OUTPUT_DIR/imposer-${SIZE}.png"
done

magick "$OUTPUT_DIR/imposer-256.png" \
  \( "$OUTPUT_DIR/imposer-48.png" \) \
  \( "$OUTPUT_DIR/imposer-32.png" \) \
  \( "$OUTPUT_DIR/imposer-16.png" \) \
  "$OUTPUT_DIR/imposer.ico"
```

Stratégie retenue :

- Committer les PNG et l'ICO pour que les builds normaux n'aient pas besoin de `rsvg-convert`, `inkscape` ou ImageMagick.
- Garder le script comme documentation exécutable de la régénération.
- CI vérifie la présence des fichiers au lieu de régénérer.

Outils locaux (machine de développement) :

- `rsvg-convert` disponible ;
- `magick` version 7.1.2-18 Q16 dans `/usr/bin/magick` ;
- Python 3 avec Pillow comme solution de repli pour l'ICO.

### 5.2. Icône de fenêtre Slint

Dans `ui/main.slint` :

```slint
export component AppWindow inherits Window {
    ...
    icon: @image-url("../assets/icons/imposer-256.png");
    ...
}
```

Choix : 256 px, format PNG, chemin relatif au fichier `.slint`. Cette icône sert de fallback et est visible dans la barre de titre de certaines plateformes.

### 5.3. Intégration Windows (`.exe`)

Dans `build.rs` Windows :

```rust
res.set_icon("assets/icons/imposer.ico");
res.set("FileDescription", "Imposition de documents PDF, PNG, JPEG et PSD");
res.set("ProductName", "Imposer");
res.set("OriginalFilename", "imposer-gui.exe");
```

L'ICO multi-résolutions est généré depuis les PNG 256, 48, 32 et 16. À ne pas oublier :

- `OriginalFilename` et `ProductName` sont visibles dans les propriétés du fichier sous Windows.
- `windows_subsystem = "windows"` dans `gui_main.rs` évite le terminal.
- `winresource` est une dépendance conditionnelle `[target.'cfg(windows)'.build-dependencies]`.

### 5.4. Intégration Linux desktop (icône hicolor)

Le script d'installation copie :

```sh
for size in 16 32 48 64 128 256 512; do
  install -m 0644 "$REPO_ROOT/assets/icons/imposer-${size}.png" \
    "$HOME/.local/share/icons/hicolor/${size}x${size}/apps/$APP_ID.png"
done
install -m 0644 "$REPO_ROOT/assets/icons/imposer.svg" \
  "$HOME/.local/share/icons/hicolor/scalable/apps/$APP_ID.svg"
```

Notes :

- Les noms de fichiers icônes reprennent `APP_ID`, pas le nom générique `imposer`.
- Installer les tailles courantes (16, 32, 48, 64, 128, 256, 512 px) ; aucune taille 24 px n'est utilisée.
- Toujours mettre à jour les caches (voir section 6).
- Le SVG scalable sert de variante scalable, et les PNG sont utilisés comme rendu immédiat.

### 5.5. Icônes UI embarquées (Adwaita dérivées)

Le projet ne dépend pas du thème d'icônes du système pour les actions. Les icônes SVG sont dans `assets/icons/ui/` :

- `file-open.svg`
- `folder-open.svg`
- `delete.svg`
- `document-save.svg`

Usage dans `ui/main.slint` :

```slint
Button {
    text: "Parcourir…";
    icon: @image-url("../assets/icons/ui/file-open.svg");
    icon-size: 16px;
    colorize-icon: true;
}

Button {
    text: "";
    icon: @image-url("../assets/icons/ui/document-save.svg");
    icon-size: 16px;
    colorize-icon: true;
    min-width: 30px;
    min-height: 30px;
}
```

Règles :

- Taille 16 px pour les icônes d'action.
- `colorize-icon: true` pour que l'icône suive la couleur de texte du contrôle, donc le thème clair/sombre.
- Copier les SVG nécessaires dans le dépôt plutôt que référencer `/usr/share/icons`.
- Garder des labels texte même avec icône (sauf boutons preset icon-only 30x30 px).
- `Parcourir…`, `Dossier…`, `Retirer la dernière`, etc. restent en français.
- Le bouton `Générer` reste text-only sans icône (choix de présentation).

---

## 6. Packaging Linux, GNOME et Wayland

### 6.1. Fichier `.desktop`

Fichier source : `packaging/linux/io.github.Franck-Demongin.Imposer.desktop`

```desktop
[Desktop Entry]
Type=Application
Name=Imposer
Comment=Imposition de documents PDF, PNG, JPEG et PSD sur des planches A4, A3 ou SRA3
Exec=imposer-gui
Icon=io.github.Franck-Demongin.Imposer
Terminal=false
Categories=Graphics;Publishing;
StartupWMClass=io.github.Franck-Demongin.Imposer
```

Notes :

- `Exec=imposer-gui` suppose que le binaire soit dans `PATH` (`~/.local/bin`).
- `Icon=` ne doit pas inclure d'extension : le moteur de thème résout le fichier.
- `Terminal=false` car l'interface est graphique.
- `Categories` est utilisé par GNOME pour classer dans le lanceur.
- Le fichier doit être validé avec `desktop-file-validate`.

### 6.2. Installation locale

`packaging/linux/install-local.sh` :

```sh
APP_ID="io.github.Franck-Demongin.Imposer"
BIN_SOURCE="${1:-$REPO_ROOT/target/release/imposer-gui}"
```

Étapes principales :

1. vérifier que le binaire existe et est exécutable ;
2. compiler d'abord `cargo build --release --features gui --bin imposer-gui` si besoin ;
3. créer `$HOME/.local/bin` ;
4. créer `$HOME/.local/share/applications` et les dossiers hicolor ;
5. installer le binaire avec `install -m 0755` ;
6. installer le `.desktop` avec `install -m 0644` sous `$APP_ID.desktop` ;
7. installer les PNG et SVG hicolor avec le nom `$APP_ID` ;
8. mettre à jour les caches.

Commandes de cache :

```sh
if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "$HOME/.local/share/applications" || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -f "$HOME/.local/share/icons/hicolor" || true
fi
```

### 6.3. Désinstallation locale

`packaging/linux/uninstall-local.sh` supprime strictement les mêmes chemins :

- binaire `$HOME/.local/bin/imposer-gui` ;
- lanceur `$HOME/.local/share/applications/$APP_ID.desktop` ;
- icônes PNG par taille et SVG scalable ;
- refresh des caches desktop/icon.

### 6.4. Point GNOME Wayland à ne pas oublier

Même si `desktop-file-validate` réussit et que les fichiers sont présents, un lanceur nouvellement installé peut ne pas apparaître dans le tableau de bord tant qu'une déconnexion/reconnexion n'a pas eu lieu. Un reboot complet n'est pas nécessaire, mais un refresh de caches seul peut ne pas suffire dans la session courante.

Procédure de test conseillée :

1. installer ;
2. exécuter `update-desktop-database` et `gtk-update-icon-cache` ;
3. vérifier avec `desktop-file-validate` ;
4. si absent dans le tableau de bord, se déconnecter/reconnecter ;
5. ne déclarer l'installation échouée qu'après ces étapes.

### 6.5. Pièges Linux hicolor

- Nom de fichier d'icône incohérent par rapport à `APP_ID` → pas d'icône dans le menu.
- `.desktop` avec `Exec` absent du PATH → clic ne lance rien.
- Cible manquante de `libwayland-dev` → Slint peut ne pas démarrer sous Wayland.
- Icône source modifiée sans régénération → SVG et PNG désynchronisés.
- Caches non rafraîchis → icônes obsolètes mais app fonctionne.

---

## 7. CI, tests et release

### 7.1. CI (`.github/workflows/ci.yml`)

Déclenchement : `push` et `pull_request`.

Matrice : `ubuntu-latest`, `windows-latest`.

Étapes :

1. checkout ;
2. toolchain Rust stable avec `rustfmt` et `clippy` ;
3. dépendances système Ubuntu, y compris `desktop-file-utils` (`libfontconfig1-dev`, `libfreetype6-dev`, `libx11-dev`, `libxkbcommon-dev`, `libwayland-dev`, `desktop-file-utils`) ;
4. étape Linux/Windows `Check tracked files and assets` (Lot 2.3), en `shell: bash` : `Cargo.lock` suit le dépôt, puis contrôle de présence des icônes pour les 7 tailles `16, 32, 48, 64, 128, 256, 512` et des fichiers `imposer.svg`, `imposer.ico`, `generate.sh`, icônes `ui/*.svg`, scripts `install-local.sh`/`uninstall-local.sh` ;
5. étape Linux uniquement `Check APP_ID consistency and Linux packaging` (Lot 2.4), en `shell: bash` : une seule déclaration de `pub const APP_ID` au niveau du module, ordre `AppWindow::new()` puis `slint::set_xdg_app_id(APP_ID)`, erreur testée et non masquée par `let _ =`, cohérence des identifiants `io.github.` et validation avec `desktop-file-validate` ;
6. `cargo fmt --check` ;
7. `cargo test --locked` ;
8. `cargo build --locked` (sans GUI) ;
9. `cargo build --features gui --locked` ;
10. `cargo clippy --all-targets --all-features --locked -- -D warnings`.

Contrôle des fichiers dans l'étape 2.3 :

```sh
git ls-files --error-unmatch Cargo.lock
test -f assets/icons/imposer-{16,32,48,64,128,256,512}.png
test -f assets/icons/imposer.svg
test -f assets/icons/imposer.ico
test -f assets/icons/generate.sh
test -f assets/icons/ui/{file-open,folder-open,delete,document-save}.svg
test -f packaging/linux/install-local.sh
test -f packaging/linux/uninstall-local.sh
```

Contrôle APP_ID dans l'étape 2.4 :

```sh
# APP_ID extrait de src/slint_ui.rs puis contrôlé avec grep/sed
# desktop '<APP_ID>.desktop', Icon/StartupWMClass, Exec=imposer-gui, Terminal=false
# scripts install/uninstall : APP_ID="<APP_ID>"
# install-local.sh : /apps/$APP_ID.png et /apps/$APP_ID.svg
# desktop-file-validate packaging/linux/<APP_ID>.desktop
```

Conséquences pour tout nouveau projet :

- Ne supprimer aucun fichier de la liste sans adapter le CI.
- `Cargo.lock` doit suivre le dépôt quand des builds `--locked` sont attendus.
- Ne renommer le `.desktop` sans mettre à jour le CI et `APP_ID`.
- Conserver le build par défaut sans GUI pour garantir la compatibilité CLI/TUI.
- Préférer `shell: bash` pour les étapes utilisant `test`, `grep` et `desktop-file-validate`, car le shell par défaut de `windows-latest` est PowerShell.

### 7.2. Release (`.github/workflows/release.yml`)

Déclenchement : tag `v*`.

Permissions : `contents: write`.

Séquence :

1. `build-windows` (`windows-latest`) :
   - fetch depth 0 pour historique git ;
   - génération de `release_notes.md` à partir du tag précédent (`git describe --tags --abbrev=0 "${tag}^"`) ;
   - `git log` des commits depuis le tag précédent ;
   - `cargo build --release --bin imposer` ;
   - `cargo build --release --features slint-ui --bin imposer-gui` ;
   - création de la release GitHub ;
   - upload `imposer.exe`, `imposer-gui.exe`.
2. `build-linux` dépend de `build-windows` :
   - `cargo build --release --bin imposer` ;
   - `cargo build --release --features slint-ui --bin imposer-gui` ;
   - ajout `imposer`, `imposer-gui` à la release existante.

Règles importantes :

- Les notes sont générées par le job Windows. Ne pas réordonner les jobs sans adapter.
- La GUI est buildée avec `slint-ui`, pas `gui` (alias équivalent côté Cargo, mais release utilise le nom long).
- Les deux binaires sont publiés pour les deux OS.
- Les assets s'appellent respectivement `imposer(.exe)` et `imposer-gui(.exe)`.

### 7.3. Publication locale / installation

Le binaire GUI n'est pas installé par défaut dans l'environnement de développement. Pour tester manuellement :

```sh
cargo build --release --features gui --bin imposer-gui
./packaging/linux/install-local.sh target/release/imposer-gui
```

Pour Windows, la validation croisée locale est limitée : la toolchain MinGW peut manquer (`x86_64-w64-mingw32-dlltool`). Dans ce cas, se fier à la CI Windows.

---

## 8. Thème clair/sombre, styles et couleurs

### 8.1. Principe retenu

L'application s'appuie sur `std-widgets` de Slint et son `Palette` pour les surfaces, bordures, contrôles et texte. Elle ne définit pas de thème clair ou sombre explicite. La palette suit donc le thème système lorsque la plateforme/Slint le permet.

Règles :

- Ne pas hard-coder de couleurs de fond/texte pour les surfaces.
- Utiliser les propriétés `Palette.*` standard (`alternate-background`, `border`, `control-background`, `foreground`, `alternate-foreground`).
- Réserver les couleurs fixes aux états sémantiques documentés.
- Garder l'alias global `Theme` minimal.

### 8.2. Tokens `Theme`

```slint
export global Theme {
    out property <length> spacing-s: 6px;
    out property <length> spacing-m: 12px;
    out property <length> spacing-l: 20px;
    out property <length> radius: 6px;
    out property <length> font-secondary: 11px;
    out property <length> font-small: 12px;
}
```

Usage typique :

```slint
Card {
    background: Palette.alternate-background;
    border-width: 1px;
    border-color: Palette.border;
    border-radius: Theme.radius;

    VerticalLayout {
        padding: Theme.spacing-m;
        spacing: Theme.spacing-m;
        ...
    }
}
```

### 8.3. Couleurs sémantiques fixes

| Couleur | Valeur | Usage |
| --- | --- | --- |
| Rouge statut | `#c42b1c` | statut niveau 2 (erreur), texte d'erreur |
| Vert statut | `#107c10` | statut niveau 1 (PDF généré / succès) |
| Overlay copie | `#33cccc33` | rectangle de placement dans la prévisualisation |
| Bord copie | `#33ccff` | bordure du placement dans la prévisualisation |
| Gris contrôle | couleurs de `Palette` via texte | zones neutres/hover |

couleurs sémantiques sans variante sombre (contraste à vérifier).
Le mapping dans Rust :

```rust
let level = if status.starts_with("Erreur") {
    2
} else if status.starts_with("PDF généré") {
    1
} else {
    0
};
ui.set_status_level(level);
```

Dans Slint :

```slint
Text {
    text: root.status;
    color: root.status-level == 2 ? #c42b1c :
           (root.status-level == 1 ? #107c10 : Palette.foreground);
}
```

### 8.4. Structure et dimensions d'écran

- Fenêtre préférée : 1100 x 720 ;
- minimum : 760 x 520 ;
- deux colonnes scrollables ;
- colonne gauche `horizontal-stretch: 3`, `min-width: 420px` ;
- colonne droite `horizontal-stretch: 2`, `min-width: 300px`, `max-width: 640px` ;
- séparateur horizontal fixe 1 px ;
- barre d'action pleine largeur en bas ;
- `ScrollView` avec `viewport-width: self.visible-width` pour éviter les débordements ;
- marge extérieure uniforme `Theme.spacing-m`.

### 8.5. Modales et empilement

Les boîtes de dialogue ne doivent pas être imbriquées au cœur du layout principal. Pattern validé :

```slint
component ModalDialog inherits FocusScope {
    z: 1000;
    width: 100%;
    height: 100%;
    ...
    Rectangle {
        width: 100%;
        height: 100%;
        background: #00000080;
        TouchArea {
            width: 100%;
            height: 100%;
            mouse-cursor: default;
            clicked => { }
        }
    }
    Rectangle {
        x: (parent.width - self.width) / 2;
        y: (parent.height - self.height) / 2;
        ...
    }
}
```

Règles :

- `ModalDialog` est un frère du conteneur principal, pas un enfant d'une carte.
- Garder `z: 1000`.
- `TouchArea` 100% x 100% pour bloquer les clics en arrière-plan.
- `FocusScope` avec initialisation de focus et capture de `Key.Escape`.
- Dimensions explicites par popup : 420x210 (custom), 420x160 (save), 420x140 (delete).
- Ne pas déplacer les boucles `changed ...` au mauvais endroit : elles restent après la fermeture du conteneur.

### 8.6. Contrôles et iconographie

- `Button` standard avec `primary: true` pour l'action principale ;
- champs `LineEdit` éditables ;
- `ComboBox` pour les listes de formats et d'orientation ;
- `CheckBox` pour booléens ;
- `ScrollView` pour les longues colonnes ;
- cartes (`Card`) avec titre optionnel ;
- textes d'aide petits, `font-size: Theme.font-secondary`, en bas de colonne droite.

### 8.7. Accessibilité

- feature `accessibility` activée ;
- `FocusScope` pour les modales ;
- touches Escape pour fermer ;
- boutons avec labels explicites ;
- status et erreur dans une zone stable et lisible.

---

## 9. Référence UI Slint : pratiques établies

### 9.1. Déclaration des propriétés et callbacks

Le composant `AppWindow` déclare :

- des propriétés en entrée `in` (état lecture seule piloté par Rust) ;
- `in-out` (formulaire bidirectionnel Slint/Rust) ;
- propriétés dérivées comme `selected-sheet-name` ;
- callbacks correspondants aux actions utilisateur ;
- une boucle `changed property => { ... }` pour réagir aux modifications.

Exemple de pattern callback :

```rust
{
    let weak = weak_ui.clone();
    ui.on_browse_source_one(move || {
        let ui = match weak.upgrade() {
            Some(ui) => ui,
            None => return,
        };
        if ui.get_busy() {
            return;
        }
        ...
    });
}
```

Règles :

- Toujours clone la `Weak` avant la closure.
- Sortir immédiatement si `busy`.
- Ne jamais bloquer dans le callback.
- Pour les dialogues de fichiers `rfd`, garder les filtres alignés aux formats supportés (`pdf`, `png`, `jpg`, `jpeg`, `psd`).
- Appeler `auto_fill_output` et `refresh_preview` après sélection source.

### 9.2. Gestion d'état multi-format de feuille

Le projet possède une mécanique de préréglages utilisateur relativement complexe. Leçons :

- différencier préréglages builtin, utilisateur et "Personnalisé" ;
- verrouiller la sélection (`sheet-selection-lock`) pendant les rebuilds ;
- consommer une fois les callbacks asynchrones de réinitialisation du ComboBox (`save_reset_consumed`) ;
- réinitialiser les timers par SingleShot 300 ms pour nettoyer les flags ;
- calculer l'index cible à partir du vecteur reconstruit, pas supposer l'état courant ;
- signaler clairement l'erreur dans le popup (`preset-save-error`) au lieu d'un panic.
- ne pas reproduire tel quel (rustines d'état ComboBox).

Si vous réutilisez ce pattern, conservez cette stratégie pour éviter les hi-jacks d'index.

### 9.3. Prévisualisation responsive

- Les placements sont des objets Slint `PreviewPlacement { x, y, w, h }` en flottants.
- Le conteneur calcule sa hauteur depuis le ratio précision (`preview-box.height = width * preview-height / preview-width`).
- Les rectangles de placement sont multipliés par l'échelle `preview-box.width / max(preview-width, 1.0)`.
- Ne pas faire ce calcul en Rust si cela force une mise à jour lourde ; la mise à l'échelle reste déclarative en Slint.

### 9.4. Status et feedback

- Statut unique `root.status`, avec `status-level` pour couleur.
- `busy` contrôle l'activation des boutons et le libellé `Générer` → `Génération…`.
- `output-ready` fait apparaître les boutons `Ouvrir le fichier` et `Ouvrir le dossier`.
- Messages en français, avec accents corrects.
- le niveau de statut déduit par `starts_with` est fragile (préférer un enum).

### 9.5. Ouvrir le fichier/dossier en sortie

Le projet inclut des actions `open_output_file`, `open_output_folder`. Bonne pratique : ne jamais ouvrir automatiquement sans action utilisateur explicite. Afficher les boutons uniquement après génération réussie.

### 9.6. Système de presets utilisateur

- validation via `imposer::presets::validate_user_preset` ;
- persistance via `imposer::presets::save_user_presets` / `load_user_presets` ;
- stockage dans le répertoire utilisateur (hors repo) ;
- merge builtin + user + "Personnalisé" dans un même vecteur `sheet_choices`.
- suppression avec confirmation dialog.

Ne pas mixer cette liste avec les sources du projet, ni committer les presets utilisateur.

### 9.7. Paramètres numériques et validation

- Textes parse avec `parse_slint_dimension` côté Rust.
- Messages d'erreur inline (`custom-error`) plutôt que modales génériques.
- Bleed et marges sécurité sont saisis en mm et transmis à la génération.

---

## 10. Pièges connus et checklist de démarrage

### 10.1. Pièges fréquents

- APP_ID local à une fonction (`APP_ID` doit être déclaré au niveau du module).
- `set_xdg_app_id` avant `AppWindow::new()` (ordre erroné sous GNOME/Wayland).
- `let _ =` qui masque l'échec de `slint::set_xdg_app_id`.
- `test -f` sous PowerShell sur `windows-latest` (préférer `shell: bash`).
- APP_ID divergent entre Rust, `.desktop` et noms d'icônes.
- Icônes committées manquantes, ce qui casse le contrôle CI.
- Build GUI absent par défaut : la feature `slint-ui`/`gui` est requise, `cargo build` seul ne compile pas le second binaire.
- Thread UI bloqué lors de la génération.
- Non-conservation des `Timer` Slint (destruction silencieuse).
- `Weak` non cloné dans les closures Slint.
- Mauvais portail de dialogue `rfd` sur Linux (oubli de `xdg-portal`/`tokio`).
- Couleurs hard-codées pour tout, ce qui casse le thème.
- Icones système supposées présentes → build non reproductible.
- `.desktop` non mis à jour dans la CI après renommage.

### 10.2. Checklist pour démarrer un nouveau projet

- [ ] décider de l'APP_ID inverse (`io.github.<user>.<App>`) avant tout fichier ;
- [ ] créer `Cargo.toml` avec `slint` optionnel et feature alias ;
- [ ] ajouter `build.rs` avec Slint compile et winresource conditionnel ;
- [ ] créer `ui/main.slint` avec Palette + tokens minimaux ;
- [ ] définir `APP_ID` et `slint::set_xdg_app_id` très tôt ;
- [ ] ajouter source SVG + script de génération + PNgs committés ;
- [ ] créer `.desktop` sous le nom `<APP_ID>.desktop` ;
- [ ] scripts install/uninstall avec refresh caches ;
- [ ] CI matrice Linux/Windows + fmt + test + clippy + présence fichiers ;
- [ ] release Windows puis Linux avec les deux binaires ;
- [ ] documentation des dépendances système ;
- [ ] tests manuels GNOME Wayland avec relogin.

### 10.3. Checklist avant publication

- [ ] `cargo fmt --check` ;
- [ ] `cargo test` ;
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` ;
- [ ] `cargo build` ;
- [ ] `cargo build --features gui` ;
- [ ] présence et cohérence des fichiers icônes ;
- [ ] `desktop-file-validate` si Linux ;
- [ ] tag `vX.Y.Z` ;
- [ ] notes release générées depuis le tag précédent.

---

## 11. Sources consultées dans Imposer

Cette référence repose sur les fichiers suivants du dépôt Imposer :

- `Cargo.toml`
- `build.rs`
- `src/gui_main.rs`
- `src/slint_ui.rs` (notamment `APP_ID`, callbacks, timer 80 ms)
- `ui/main.slint` (tokens, Palette, layout, modales)
- `assets/icons/generate.sh`
- `packaging/linux/install-local.sh`
- `packaging/linux/uninstall-local.sh`
- `packaging/linux/io.github.Franck-Demongin.Imposer.desktop`
- `.github/workflows/ci.yml`
- `.github/workflows/release.yml`
- mémoires projet sur GNOME Wayland, APP_ID/fichier `.desktop`, palette et référence graphique

---

> Note : `doc/` et `.qwen/` sont ignorés par Git dans Imposer. Ce document est une note de travail interne et ne doit pas être ajouté au dépôt.