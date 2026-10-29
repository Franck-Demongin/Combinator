# Référence graphique GUI — style portable

Référentiel pour réutiliser le style graphique d'un projet Slint/Rust dans un autre projet.
Les valeurs ci-dessous sont celles du composant exporté `MainWindow` dans `crates/cli/ui/main_window.slint`, associé au cœur applicatif GUI dans `crates/cli/src/gui.rs`.

Les noms d'application, chemins d'icônes et identifiants XDG sont présentés sous des formes génériques (`ProjectName`, `project_name`, `io.github.<auteur>.ProjectName`).

## 1. Version et prérequis

| Élément | Valeur |
|---|---|
| Slint | `1.18.1` (voir `Cargo.lock`) |
| Widgets | `std-widgets.slint` de Slint |
| Thème | `Palette` Slint, complété par un global `Theme` |
| Backend cible vérifié | winit + rendu `femtovg` |
| Mode | feature Cargo `gui` |
| Composants hors feature | le comportement hors GUI reste dans le crate noyau (`project_name-core`) |

Principes retenus :

- un fichier `.slint` par vue ou composant ;
- tokens et tailles centralisés dans `global Theme` ;
- couleurs de `Palette` Slint pour toutes les surfaces, textes et bordures standards ;
- accent sémantique uniquement pour les notifications toast ;
- logique d'état côté Rust ;
- aucune dépendance à un thème Slint personnalisé complètement redéfini.

Cette référence ne couvre que l'interface actuellement portée dans le dépôt. Elle n'est pas une bibliothèque exportable promise stable.

## 2. Tokens graphiques

### 2.1 Espacements et rayons

Toutes les valeurs sont en pixels Slint.

| Token | Valeur | Usage |
|---|---:|---|
| `Theme.spacing` | `8px` | espacement standard, padding des boîtes, gaps |
| `Theme.corner-radius` | `6px` | rayon des toasts et cartes thématiques |
| rayon fixe du badge de fermeture | `12px` | badge circulaire 24x24 |
| toast max width | `360px` | largeur maximale d'un toast |

### 2.2 Dimensions de fenêtre

| Token | Valeur |
|---|---:|
| `Theme.window-preferred-width` | `860px` |
| `Theme.window-preferred-height` | `520px` |
| `Theme.window-min-width` | `760px` |
| `Theme.window-min-height` | `460px` |

### 2.3 Dialogues

| Token | Valeur |
|---|---:|
| `Theme.dialog-width` | `760px` |
| `Theme.dialog-height` | `500px` |
| `Theme.dialog-vertical-margin` | `45px` |

### 2.4 Champs et cartes

| Token | Valeur | Usage |
|---|---:|---|
| `Theme.label-min-width` | `0px` | réservé ; aucune largeur minimale n'est imposée pour les libellés compacts |
| `Theme.crop-field-min-width` | `120px` | largeur minimale des champs Largeur crop et Hauteur crop |
| `Theme.frame-min-height` | `240px` | hauteur minimale du cadre de rapport |

### 2.5 Global source

Extrait fidèle du fichier actuel :

```slint
global Theme {
    out property <length> spacing: 8px;
    out property <length> window-preferred-width: 860px;
    out property <length> window-preferred-height: 520px;
    out property <length> window-min-width: 760px;
    out property <length> window-min-height: 460px;
    out property <length> dialog-width: 760px;
    out property <length> dialog-height: 500px;
    out property <length> dialog-vertical-margin: 45px;
    out property <length> label-min-width: 0px;
    out property <length> crop-field-min-width: 120px;
    out property <length> frame-min-height: 240px;
    out property <length> corner-radius: 6px;
}
```

## 3. Palette et couleurs

### 3.1 Surfaces standards

Les surfaces suivantes utilisent `Palette` Slint, donc le rendu suit le thème système actif.

| Usage | Token Slint |
|---|---|
| fond de carte standard | `Palette.alternate-background` |
| bordure standard | `Palette.border` |
| texte de rapport | `Palette.foreground` |
| texte secondaire de l'en-tête | `Palette.foreground` |

Ne pas réintroduire des valeurs codées en dur de type `#f7f7f7`, `#9a9a9a` ou `#202020` pour le cadre de rapport. Les tests et historiques du projet unifient ce cadre avec `Palette`.

### 3.2 Toasts : accents sémantiques vivants

Les toasts ne suivent pas les nuances neutres du thème. Ils utilisent des couleurs vives non pastel avec texte à fort contraste.

| Niveau | Signification | Barre | Bordure et badge de fermeture | Texte |
|---:|---|---|---|---|
| 0 | succès / info | `#2e9f54` | `#1d6f3a` | `#ffffff` |
| 1 | avertissement | `#f08c00` | `#a66100` | `#111111` |
| 2 | erreur | `#e53935` | `#9b000e` | `#ffffff` |

Règles complémentaires :

- le badge de fermeture d'un toast est une pastille circulaire 24x24, couleur de bordure associée, avec `×` blanc ;
- le texte du toast conserve `wrap: word-wrap` ;
- la largeur maximale reste `360px` ;
- l'icône et le badge restent alignés dans une `HorizontalBox` ;
- la couleur n'indique pas seulement le niveau : le texte explicatif reste dans la charge utile Slint/Rust.

### 3.3 Surcouches et overlay

| Élément | Valeur |
|---|---|
| scrim de rapport | rectangle plein calque `#00000080` |
| fond de toast overlay | `transparent` |

### 3.4 Palette interdite pour les toasts

Pour conserver le contraste restauré dans les notifications, ne pas remplacer les toasts par des couleurs `Palette` neutres ou atténuées.

## 4. Typographie

| Usage | Taille | Graisse | Couleur | Famille |
|---|---:|---:|---|---|
| titre En-tête ProjectName | `16px` | `700` | défaut du widget `Text` | défaut |
| sous-titre d'en-tête | `12px` | défaut | `Palette.foreground` | défaut |
| rapport monospace | `12px` | défaut | `Palette.foreground` | `"monospace"` |
| autre texte standard | défaut `Text` Slint | défaut | défaut | défaut |
| libellé compact de champ (`Dossier`, `Fichier`, `Feuille — …`, etc.) | taille par défaut de `Text` Slint | défaut | défaut | défaut |
| libellé secondaire de synthèse (`Dossier :`, `Fichier :`, etc.) | `12px` (`Theme.font-small`) | défaut | `Palette.alternate-foreground` | défaut |

L'en-tête compact utilise un icône image de 48x48 placé à gauche du bloc titre/sous-titre. Le libellé window "ProjectName" est stocké dans la propriété `title` de la fenêtre, pas dans une étiquette du layout principal.

## 5. Organisation visuelle générale


- Fenêtre sans cadre graphique global autour de tout le contenu.
- Une barre d'en-tête compacte occupe le haut.
- Les zones fonctionnelles sont trois cartes thématiques empilées.
- Un espacement de `8px` sépare les éléments principaux.
- Un bloc flexible invisible occupe l'espace vertical libre et maintient le pied de page en bas.
- Un séparateur horizontal de 1 px (`Palette.border`) précède le pied de page.
- Le pied de page reste visible à tout moment pendant l'interaction : ouverture de fichier/dossier à gauche, action principale à droite.

## 6. Composants de layout

### 6.1 Petite en-tête compact

Usage : identification visuelle et contexte au sommet de la fenêtre.

Structure reconstituée :

```slint
HorizontalBox {
    vertical-stretch: 0;
    alignment: stretch;
    spacing: Theme.spacing;

    Image {
        source: @image-url("../../../assets/icons/project_name-64.png");
        width: 48px;
        height: 48px;
        image-fit: contain;
    }

    VerticalBox {
        padding: 0px;
        spacing: 0px;

        Text {
            text: "ProjectName";
            font-size: 16px;
            font-weight: 700;
        }
        Text {
            text: "Sous-titre de l'application";
            font-size: 12px;
            color: Palette.foreground;
            horizontal-alignment: left;
        }
    }
}
```

Règles :

- ne pas rajouter de surface haute ou de cadre autour de l'en-tête sans validation ;
- l'icône source est `assets/icons/project_name-64.png` ;
- l'icône vectorielle source de référence est `assets/icons/project_name.svg`.

### 6.2 Carte thématique

C'est le bloc le plus réutilisable de l'interface portée : fond `Palette.alternate-background`, bordure `Palette.border`, rayon standard, padding standard.

```slint
Rectangle {
    background: Palette.alternate-background;
    border-color: Palette.border;
    border-width: 1px;
    border-radius: Theme.corner-radius;
    vertical-stretch: 0;
    horizontal-stretch: 1;
    height: self.preferred-height;

    VerticalBox {
        padding: Theme.spacing;
        spacing: Theme.spacing;
        vertical-stretch: 0;

        /* contenu */
    }
}
```

Règles importantes :

- `height: self.preferred-height;` est nécessaire. Un `Rectangle` Slint tend à remplir son parent s'il n'est pas contraint ;
- ne pas transformer une carte en composant global ;
- éviter de dupliquer un cadre extérieur supplémentaire.

### 6.3 Étiquettes empilées

Les libellés `Source`, `Sortie`, `Largeur crop (mm)` et `Hauteur crop (mm)` sont placés sur leur propre ligne au-dessus des champs.

Source et sortie :

```slint
HorizontalBox {
    spacing: Theme.spacing;
    alignment: stretch;
    vertical-stretch: 0;

    LineEdit {
        text <=> root.source-path;
        placeholder-text: "Chemin du PDF source";
        horizontal-stretch: 1;
    }
    Button {
        text: "Parcourir...";
        horizontal-stretch: 0;
        clicked => { root.browse-source(); }
    }
}
```

Recadrage :

```slint
LineEdit {
    text <=> root.crop-width-mm;
    placeholder-text: "85";
    horizontal-stretch: 1;
    min-width: Theme.crop-field-min-width;
}
```

Règles :

- utiliser `Parcourir...`, pas une ancienne formulation ;
- les deux champs de recadrage ont la même largeur minimale ;
- la barre des dimensions utilise le même espacement standard.

### 6.4 Pied de page et actions permanentes
Pied de page permanent, situé sous les cartes principales et le séparateur de 1 px. Il reste visible pendant toute l'interaction et ne défile pas avec le contenu principal.

Ordre tel qu'il figure dans le Slint actuel, de gauche à droite :

1. `Rapport`, si `root.report-available` ;
2. `Ouvrir le fichier`, si `root.output-available` ;
3. `Ouvrir le dossier`, si `root.output-available` ;
4. `Recadrer`, action principale `primary: true`.

Règles :

- `Recadrer` reste à droite dans `alignment: end` ;
- les actions conditionnelles se placent à gauche de l'action principale ;
- `Rapport` doit être le plus à gauche, puis les actions d'ouverture ;
- la barre utilise la même `HorizontalBox` standard.
- un séparateur horizontal de 1 px (`Palette.border`) précède le pied de page ;
- les boutons secondaires et d'ouverture sont en `horizontal-stretch: 0` et calés à gauche ;
- un `Rectangle { horizontal-stretch: 1; }` placé avant l'action principale la force à droite ;
- l'action principale utilise `primary: true` avec `horizontal-stretch: 0`.

### 6.5 Grille compacte de champs et libellés

Les libellés primaires de formulaires sont plus compacts dans Imposer qu'auparavant : ce sont des `Text` simples, sans padding direct ni largeur minimale imposée.

```slint
GridLayout {
    spacing-horizontal: Theme.spacing;
    spacing-vertical: Theme.spacing;

    Text { text: "Dossier"; row: 0; col: 0; vertical-alignment: center; }
    LineEdit { text <=> root.output-dir; row: 0; col: 1; horizontal-stretch: 1; }
    Button { text: "Dossier…"; icon: @image-url("../assets/icons/ui/folder-open.svg"); icon-size: 16px; colorize-icon: true; enabled: !root.busy; row: 0; col: 2; min-width: 100px; }

    Text { text: "Fichier"; row: 1; col: 0; vertical-alignment: center; }
    LineEdit { text <=> root.output-name; row: 1; col: 1; horizontal-stretch: 1; }

    Text { text: "Feuille — " + root.selected-sheet-name; row: 2; col: 0; }
    Text { text: "Orientation feuille"; row: 2; col: 1; }
}
```

Règles :

- les libellés primaires de champ sont des `Text` simples ;
- aucun `padding` ne leur est appliqué directement ;
- aucune largeur minimale n'est imposée via `label-min-width` ;
- l'espacement horizontal et vertical de la grille est `Theme.spacing` ;
- `vertical-alignment: center` est utilisé pour les libellés accompagnant des champs ou boutons ;
- le padding standard de `8px` reste porté par la carte conteneur ;
- ne pas réintroduire un `min-width` ou un padding sur les libellés sans raison fonctionnelle.

## 7. Composant Slint `Snackbar`

C'est un composant réellement exporté au niveau du fichier. Il reste compact, fermable, et se fie à une palette sémantique vive.

### 7.1 Extrait à réutiliser

```slint
component Snackbar inherits Rectangle {
    in property <string> text: "";
    in property <int> level: 0;
    in property <int> id: 0;

    callback close-requested();

    background: self.level == 2 ? #e53935 : (self.level == 1 ? #f08c00 : #2e9f54);
    border-color: self.level == 2 ? #9b000e : (self.level == 1 ? #a66100 : #1d6f3a);
    border-width: 1px;
    border-radius: Theme.corner-radius;
    height: self.preferred-height;

    HorizontalBox {
        padding: Theme.spacing;
        spacing: Theme.spacing;
        alignment: stretch;

        Text {
            text: root.text;
            color: root.level == 2 ? #ffffff : (root.level == 1 ? #111111 : #ffffff);
            wrap: word-wrap;
            horizontal-alignment: left;
            vertical-alignment: top;
            horizontal-stretch: 1;
        }

        Rectangle {
            width: 24px;
            height: 24px;
            horizontal-stretch: 0;
            border-radius: 12px;
            background: root.level == 2 ? #9b000e : (root.level == 1 ? #a66100 : #1d6f3a);

            Image {
                x: 5px;
                y: 5px;
                width: 14px;
                height: 14px;
                source: @image-url("../../../assets/icons/ui/close.svg");
                image-fit: contain;
            }

            TouchArea {
                width: 24px;
                height: 24px;
                clicked => {
                    root.close-requested();
                }
            }
        }
    }
}
```

### 7.2 Règles d'implémentation Slint

- le composant doit être un `Rectangle` dont `height: self.preferred-height;` est explicitement défini ;
- la largeur est fixée par l'overlay, pas dans le `Snackbar` ;
- `horizontal-stretch: 1` va au texte, `0` au badge ;
- le badge contient une `TouchArea` plein écran du rectangle ;
- utiliser une callback anonyme `close-requested()` et identifier le toast côté Rust par son ID.

### 7.3 Overlay à l'écran

Placement actuel : coin supérieur droit.

```slint
Rectangle {
    visible: root.snackbar-items.length > 0;
    width: min(360px, max(0px, root.width - Theme.spacing * 4));
    height: self.preferred-height;
    x: max(Theme.spacing * 2, root.width - self.width - Theme.spacing * 2);
    y: Theme.spacing * 2;
    background: transparent;

    VerticalBox {
        spacing: Theme.spacing;
        for i in root.snackbar-items.length : Snackbar {
            /* mapping du modèle, ordre inversé pour empiler */
        }
    }
}
```

Règles :

- l'overlay aussi déclare `height: self.preferred-height;` ;
- au moins un toast doit être visible pour activer l'overlay ;
- la marge du toit de fenêtre est `16px` (`Theme.spacing * 2`) ;
- la largeur reste plafonnée à `360px`.

## 8. Comportement des toasts côté Rust

Le cœur applicatif associe la présentation Slint à une file simple. Les détails sont dans `crates/cli/src/gui.rs`.

| Contrat | Valeur |
|---|---|
| nombre maximal visible | `3` |
| compartiment source | `SnackbarQueue` |
| identifiant | `u32`, encapsulé en `i32` vers Slint |
| ordre d'affichage | ancien vers nouveau |
| disparition automatique | `4 secondes` par toast |
| fermeture manuelle | badge de fermeture |
| synchronisation | `VecModel` reconstruit à partir de la file |

Règles de réutilisation :

- chaque toast conserve son propre timer ;
- les toasts émis simultanément ne se recouvrent pas ;
- la file nettoie les toasts jusqu'à 3 visibles ;
- les identifiants sont parallèles à la pile pour éviter les fermetures erronées.

Ne pas revenir à une seule bannière de statut si l'objectif est de conserver ce comportement dans un autre projet.

## 9. Rapport

### 9.1 Comportement global

- Le rapport s'ouvre dans un `PopupWindow` plein écran dont `close-policy: no-auto-close` ;
- un rectangle semi-transparent `#00000080` crée le scrim ;
- un `TouchArea` plein calque absorbe les clics hors dialogue ;
- un `Dialog` Slint standard est centré et stocke le titre `root.report-title` ;
- Escape ferme via un `FocusScope` d'insertion ;
- les clics à l'intérieur du contenu sont absorbés par un `TouchArea` dédié ;
- la fermeture est déclenchée par un `StandardButton { kind: close; }` ;
- `root.report-visible` suit l'état exact de l'overlay.

### 9.2 Dimensions du dialogue

```text
width = root.width >= 760 ? 760 : root.width
height = root.height >= 520 ? 500 : root.height - 45
```

### 9.3 Cadre de rapport

```slint
Rectangle {
    background: Palette.alternate-background;
    border-color: Palette.border;
    border-width: 1px;
    border-radius: Theme.corner-radius;
    horizontal-stretch: 1;
    vertical-stretch: 1;
    min-height: Theme.frame-min-height;
    clip: true;

    ScrollView {
        horizontal-stretch: 1;
        vertical-stretch: 1;
        content-width: self.visible-width;
        horizontal-scrollbar-policy: always-off;

        VerticalBox {
            padding: Theme.spacing;
            spacing: 0px;
            alignment: stretch;

            Text {
                text: root.report;
                width: 100%;
                min-width: 0px;
                wrap: word-wrap;
                vertical-alignment: top;
                horizontal-alignment: left;
                font-family: "monospace";
                font-size: 12px;
                color: Palette.foreground;
            }
        }
    }
}
```

Règles précises :

- le padding ne doit pas être appliqué directement à un `Text` ; il est dans le `VerticalBox` ;
- `ScrollView.content-width` doit être `self.visible-width` dans ce schéma ;
- `Text.min-width: 0px` évite un débordement horizontal ;
- la barre horizontale de défilement doit rester désactivée pour ce rapport ;
- limiter les dimensions du `Dialog`, et non du rectangle conteneur, pour éviter le débordement.
- `clip: true` conserve le rapport dans le cadre.

### 9.4 Logique Rust du rapport

Règles à préserver pour garder la même présentation :

- `format_report_body(_kind, raw)` normalise d'abord les lignes blanches ;
- `annotate_json_values` prétraite les blocs JSON visibles dans le rapport ;
- `format_json_value` pretty-print le JSON valide ;
- un JSON invalide reste affiché tel quel après trim ;
- `set_report(...)` place titre et corps, puis marque le rapport disponible ;
- le rapport crop actuel reste volontairement concis : fichier source, fichier généré, succès, dimensions, pagination, couleur source, profils ICC, profils de sortie, taille de fichier, validation, avertissements ;
- l'interface n'affiche que les éléments de ce cadre et n'introduit pas une seconde zone de rapport.

## 10. Identité application et icônes

| Élément | Valeur |
|---|---|
| identifiant XDG | `io.github.<auteur>.ProjectName` |
| nom d'application | `ProjectName` |
| titre fenêtre | `ProjectName` |
| fichier bureau | `packaging/linux/project_name.desktop` |
| catégories bureau | `Office;` |
| icône vectorielle de référence | `assets/icons/project_name.svg` |
| icône GUI dans le layout | `assets/icons/project_name-64.png` |
| icône binaires | `assets/icons/project_name-16.png` à `project_name-512.png`, ICO |
| instance Slint | `slint::set_xdg_app_id(...)` appelé après création du composant, avant affichage |

L'icône et le nom doivent accompagner toute tentative de portage, même si les valeurs se changent pour un autre projet.

## 11. Contrat Slint / Rust exporté

Propriétés principales exportées par le composant racine :

| Nom | Type |
|---|---|
| `source-path` | `string` |
| `output-path` | `string` |
| `crop-width-mm` | `string` |
| `crop-height-mm` | `string` |
| `report-title` | `string` |
| `report` | `string` |
| `output-available` | `bool` |
| `report-available` | `bool` |
| `report-visible` | `bool` |
| `snackbar-items` | `[SnackbarItem]` |

Callbacks principaux :

- `browse-source()`
- `browse-output()`
- `crop()`
- `open-output()`
- `open-output-folder()`
- `open-report()`
- `close-report()`
- `close-snackbar(int)`

Composant structuré Slint :

```slint
struct SnackbarItem {
    text: string,
    level: int,
    id: int,
}
```

## 12. checklist de portage vers un autre projet

1. Conserver les tokens de base dans un global Slint identique.
2. Utiliser `Palette` pour les états standards avant d'envisager d'autres couleurs.
3. N'ajouter des couleurs codées qu'à des situations sémantiques précises comme les toasts.
4. Conserver `height: self.preferred-height;` sur chaque composant et overlay flottants.
5. Porter ensemble la file de toasts, ses timers, le plafond à 3 et les identifiants.
6. Refaire le rapport comme un seul bloc scrollable, monospace, padé par l'extérieur.
7. Arrêter tout mouvement latéral avec `content-width: self.visible-width` et `min-width: 0px`.
8. Garder l'overlay modal `#00000080`, l'escape et la fermeture par bouton standard.
9. Maintenir les libellés empilés au-dessus des champs.
10. Conserver le pied de page permanent : séparateur de 1 px, boutons d'ouverture à gauche, filler horizontal, action principale à droite.
11. Vérifier les commandes clippy/test et l'affichage sur thème clair et sombre.
12. Conserver les libellés primaires de formulaires compacts, sans padding direct, avec espacement de grille à `Theme.spacing`, sauf exception explicite.

## 13. Garde-fous explicites

- Ne pas réintroduire `content-width: 100%` dans le rapport ;
- ne pas appliquer `padding` directement sur `Text` ;
- ne pas réintroduire les couleurs de rapport codées obsolètes ;
- ne pas remplacer les toasts vifs par des nuances neutres ;
- ne pas faire déborder la barre horizontale de rapport ;
- ne pas ajouter de second cadre de rapport ;
- ne pas retirer un global de tokens partagé ;
- ne pas supprimer le séparateur de 1 px avant le pied de page ;
- ne pas remplacer le filler horizontal par un alignement ambigu ;
- ne pas faire disparaître le pied de page pendant la navigation ou le défilement ;
- ne pas ajouter de style graphique décoratif au-delà de cette référence, en accord avec la spécification du projet porté.

## 14. Icônes UI et boutons

Les actions principales utilisent des SVG Adwaita (thème GNOME) en 16 px, copiés dans `assets/icons/ui/`. Un bouton est composé ainsi:

```slint
Button {
    text: "Ouvrir le fichier";
    icon: @image-url("../../../assets/icons/ui/document-open.svg");
    icon-size: 16px;
    colorize-icon: true;
    horizontal-stretch: 0;
    clicked => { root.open-output(); }
}
```

Icônes retenues:

| Usage | Fichier |
|---|---|
| source PDF, ouvrir un fichier | `assets/icons/ui/document-open.svg` |
| sortie et ouvrir un dossier | `assets/icons/ui/folder-open.svg` |
| Rapport | `assets/icons/ui/document-properties.svg` |
| fermeture d'un toast | `assets/icons/ui/close.svg` (rendu blanc sur le badge) |
| action `Recadrer` | pas d'icône, par choix |
| fenêtre, icônes du système | `assets/icons/project_name-*.png`, `project_name.ico` et `project_name.svg` inchangés |

Règles:

- `colorize-icon: true` permet au thème de recolorier un SVG sombre (`#2e3436`) pour l'adapter au texte/bouton;
- pour le toast, l'icône blanche est dans un SVG dédié placé dans le badge circulaire;
- ne pas réutiliser `delete.svg` pour une erreur si l'action n'est pas une suppression;
- garder la barre d'actions dans l'ordre existant (Rapport, Ouvrir le fichier, Ouvrir le dossier, Recadrer).

## 15. Sources dans le dépôt

- `crates/cli/ui/main_window.slint`
- `crates/cli/src/gui.rs`
- `crates/cli/src/lib.rs`
- `crates/cli/build.rs`
- `assets/icons/`
- `packaging/linux/project_name.desktop`
- `QWEN.md`
- mémoires de projet associées au layout et au thème de l'interface portée

Version documentée : Slint `1.18.1`, projet exemple `0.2.3`.
",