---
title: Profil technique du projet cible
version: 0.1
status: brouillon # brouillon | validé | déprécié (passer à `validé` après la première validation stricte verte)
---

# Profil technique — `<nom-du-projet>`

> Document de référence pour l'Analyste, le Développeur et le QA.
> Racine absolue du projet cible : `<ROOT_DIR>` (doit rester identique aux écritures).
> À compléter à partir des préférences globales (`~/.qwen/QWEN.md`) et des choix du projet.

## 1. Langage et version

- Langage :
- Version minimale / recommandée :
- Gestionnaire / runtime :

## 2. Outils imposés ou préférés

- Gestion des dépendances :
- Exécution :
- Lint :
- Formatage :
- Tests :
- Build :
- Documentation :

## 3. Bibliothèques

### Préférées (par défaut)

- 

### Exclues (et pourquoi)

- 

## 4. Structure attendue

```text
.
├── ...
└── ...
```

## 5. Conventions de code

- Style :
- Typage :
- Nommage :
- Organisation des modules :

## 6. Conventions de design / interface

- Interface : CLI / GUI / web / autre
- Bibliothèque visuelle / composants :
- Accessibilité :
- Messages et sortie utilisateur :

## 7. Commandes de validation stricte

Depuis la racine du projet :

- Tests : `commande`
- Lint : `commande`
- Build : `commande`
- Vérification supplémentaire : `commande` (optionnel)

## 8. Cas d'usage typiques

- 

## 9. Décisions et références

- ADR :
- Liens :

## 10. Notes de migration / changement de profil

- Ce profil évolue par PR/commit documenté.
- Une rupture majeure (changement de stack ou d'outil) doit faire l'objet d'un ADR.