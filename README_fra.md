<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-AUTOLEVEL banner" width="100%">
</p>

# 📖 EH3D-AUTOLEVEL

<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | 🇫🇷 <b>Français</b> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### Corrige le Z d'un programme G-code avec la carte de hauteurs de la table

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.2-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Vérification d'honnêteté - ce qui fonctionne aujourd'hui :** une bibliothèque Rust et un programme en ligne de commande (`eh3d-autolevel`) avec 28 tests exécutés sur un ordinateur : corrige le Z d'un programme G-code avec la carte de hauteurs de la table, en coupant les mouvements en morceaux qui suivent la surface. Il n'a pas encore de fenêtre, n'a pas été essayé sur une machine et n'a pas été vérifié sur une collection de fichiers réels.

---

> **État de sécurité :** ce programme ne commande aucune machine et n'est pas un système de sécurité. Ce qu'il produit doit être vérifié par une personne avant tout essai sur une machine ; rien ici n'a fonctionné sur du matériel réel.


## 📋 En un coup d'œil

| | |
|---|---|
| Version | `0.0.2` |
| Maturité | Échafaudage |
| Tests | 28 |
| Pile technique | Rust |
| Sur du matériel réel | Non essayé : rien ici n'a fonctionné sur une machine |

**Aller à  :** [Présentation](#-présentation) · [Architecture](#-architecture) · [Démarrage rapide](#-démarrage-rapide) · [Tests](#-tests) · [Structure](#-structure-du-dépôt) · [Liés](#-projets-liés)

## 🚦 État du projet

Chaque ligne est une partie de ce dépôt et le feu dit où elle en est aujourd'hui. Elle est écrite d'après ce que le dépôt peut montrer (tests, compilations, exécutions), pas d'après ce qui est prévu.

🟢 Fait et vérifié · 🟡 Fonctionne, avec des limites connues · 🟠 À moitié fait ou pas encore vérifié · 🔴 Pas fait ou ne fonctionne pas

| Domaine | État | Ce que cela veut dire ici |
|---|---|---|
| Cœur | 🟡 | corrige le Z d'un programme G-code avec la carte de hauteurs de la table, en coupant les mouvements en morceaux qui suivent la surface ; testé sur un ordinateur avec des entrées normales et abîmées, pas avec une collection de fichiers réels. |
| Ligne de commande | 🟡 | `eh3d-autolevel` exécute les mêmes fonctions que la bibliothèque et affiche le résultat ; chaque erreur revient sous forme de message et de code de sortie non nul. |
| Tests | 🟡 | 28 tests passent (`cargo test`), avec des cas normaux, des cas limites et des entrées abîmées. |
| Sécurité | 🟡 | Rust sûr et simple (`forbid(unsafe_code)`) : rien ne panique sur les fichiers reçus et une mauvaise valeur est une erreur avec sa raison ; pas de revue indépendante. |
| Fenêtre | 🔴 | Prévu : la fenêtre egui commune à la suite ; aujourd'hui il n'y a que la ligne de commande. |
| Matériel réel | 🔴 | Non essayé : rien ici n'a été exécuté sur une machine. |
| Documentation | 🟡 | README en huit langues, le journal des modifications et les deux images dessinées depuis `docs/`. |

## 🎯 Présentation

* **Le programme** (`src/gcode.rs`) : un mouvement de coupe (G1) est coupé en morceaux d'au plus la longueur choisie et chaque morceau reçoit le Z que donne la carte pour sa fin ; un mouvement rapide (G0) n'est corrigé que s'il a un Z. Le zéro de la correction est un point de référence, celui où le Z du travail a été fixé.
* **La surface** (`src/surface.rs`) : bilinéaire (celle de la carte d'EH3D-COMMON) ou bicubique (Catmull-Rom), qui passe par chaque point mesuré et suit mieux une table courbe ; hors de la grille, la hauteur du bord le plus proche est utilisée.
* **Ce qui est refusé**, avec le numéro de ligne : distances relatives (G91), pouces (G20), arcs (G2, G3), un mouvement avant tout G0 ou G1, un nombre qui n'en est pas un, une coordonnée au-delà d'un million de millimètres, un mouvement qui donnerait plus de cent mille morceaux et une carte aux points non mesurés.
* **Ligne de commande stricte :** une option mal écrite (`--powr`) ou un mot en trop est une erreur qui la nomme, et non quelque chose d'ignoré ; sinon le programme ferait sa sortie avec les valeurs par défaut.
* **Jamais nivelé deux fois :** le programme corrigé commence par une ligne de commentaire (`(eh3d-autolevel VERSION map=HASH ref=X,Y méthode seg=N)`) qui dit qu'il a été nivelé et avec quelle carte ; un programme qui la porte est refusé avec sa ligne, car le corriger à nouveau ajouterait deux fois la correction de la table.
* **Les codes qui déplacent l'origine sont refusés :** G10, G28, G30, G38, G43, G53, G92 et tout autre code G qui n'est ni un simple mouvement ni un réglage laissant la position inchangée arrêtent le programme avec le numéro de la ligne (ils étaient transformés en mouvements vers Z0) ; une temporisation (G4) et la ligne `%` du début et de la fin d'un programme passent telles quelles, et une carte sans hauteur finie est refusée au lieu d'écrire `Z NaN`.
* **`--dry-run` :** `apply` calcule le résultat et le compte (mouvements, morceaux ajoutés, décalage de Z) sans écrire le fichier de sortie, qui peut alors être omis.

## 🧩 Rôle dans EH3D

EH3D-AUTOLEVEL est un programme de l'EH3D Fabrication Suite. Aujourd'hui il fonctionne comme bibliothèque et ligne de commande ; sa fenêtre, avec l'aspect commun aux autres programmes de la suite, est prévue. Il lit et écrit des fichiers simples et peut donc s'utiliser seul ou entre les autres programmes.

## 🏛️ Architecture

<p align="center">
  <img src="images/architecture.svg" alt="Architecture de EH3D-AUTOLEVEL : qui l'utilise, ce qu'il contient et de quoi il dépend" width="100%">
</p>


Comment les parties de ce dépôt s'articulent : qui l'utilise, ce qu'il contient et de quoi il dépend. L'image est dessinée depuis `docs/architecture.json` et vérifiée avec le code.

## ✨ Fonctionnalités

* **Une bibliothèque et une ligne de commande :** chaque commande de `eh3d-autolevel` est une fonction de la bibliothèque ; un autre programme de la suite peut l'appeler sans lancer de processus.
* **Tests avec entrées abîmées :** les tests fournissent aussi des fichiers vides, du bruit, des nombres énormes et des fichiers coupés, et aucun ne doit paniquer.
* **Sans deviner :** une valeur absurde est une erreur qui dit laquelle et pourquoi, jamais un résultat qui n'en a que l'air.
* **Pas de code unsafe :** `#![forbid(unsafe_code)]` dans la bibliothèque et le programme.

## 🔒 Sécurité et protection

| Menace | Défense |
|---|---|
| Un fichier abîmé, coupé ou vide | pas de panique : les tests coupent une entrée valide à chaque position et fournissent du bruit |
| Un nombre absurde (négatif, non fini, démesuré) | refusé avec une erreur qui nomme la valeur ; rien n'est arrondi en quelque chose de plausible |
| Usage non sûr de la mémoire | Rust sûr et simple avec `forbid(unsafe_code)` |

**Ce qui n'est pas défendu :** un résultat produit sans erreur n'est pas un résultat vérifié. Rien ici ne connaît les limites d'une machine et le programme n'a pas eu de revue indépendante. Signalez une vulnérabilité en privé (voir [SECURITY.md](SECURITY.md)).

## 🔌 Interfaces

La bibliothèque est `eh3d_autolevel` (`use eh3d_autolevel::...`) ; la ligne de commande est `eh3d-autolevel`. Les commandes sont celles-ci ; chacune affiche son résultat sur la sortie standard et signale une erreur sur la sortie d'erreur avec un code de sortie non nul. Il n'y a pas d'accès réseau.

```text
EH3D-AUTOLEVEL/
├── src/
│   ├── lib.rs                                   la bibliothèque : ce que chaque commande utilise
│   ├── main.rs                                  la ligne de commande
│   ├── cli_args.rs                              les arguments de la ligne de commande, le même fichier dans chaque petit utilitaire
│   ├── gcode.rs                                 une partie de la logique
│   └── surface.rs                               une partie de la logique
├── tests/
│   ├── autolevel.rs                             tests de la bibliothèque et de la ligne de commande
│   ├── dry_run.rs                               tests de --dry-run : compte la même chose et n'écrit rien
│   ├── readme.rs                                tests qui exécutent ce que promettent le README et les scripts run
│   └── strict.rs                                tests de la marque d'un programme nivelé et de la ligne de commande stricte
├── examples/
│   ├── sample.nc                                un exemple pour essayer le programme
│   └── table.txt                                un exemple pour essayer le programme
├── docs/
│   ├── architecture.json                        données de l'image d'Architecture
│   └── structure.json                           données de l'image de Structure
├── tools/                                       contrôle de la CI et du README et outils d'images
├── images/                                      bannière et images d'Architecture et de Structure du dépôt
├── .github/                                     workflows de CI et de publication, modèles d'issues et de pull requests
├── eh3d.project.json                            manifeste : version, rôle, maturité et commande de compilation
├── Cargo.toml Cargo.lock                        le workspace et ses dépendances verrouillées
├── CHANGELOG.md                                 ce qui a changé dans chaque version
├── SECURITY.md SUPPORT.md                       signaler une vulnérabilité et poser des questions
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   contribuer, code de conduite et licence
```

## ⚙️ Configuration

Il n'y a pas encore de fichier de configuration : ce que la personne choisit est donné en arguments de la ligne de commande (voir Interfaces) et chaque valeur par défaut est écrite dans l'aide de la commande (`eh3d-autolevel help`). Les limites fixes sont des constantes nommées dans le code, couvertes par des tests.

## 🚀 Démarrage rapide

Il faut une chaîne d'outils Rust stable (1.75 ou plus récente).

```bash
cargo build --release
cargo test
cargo run --release -- info examples/table.txt
```

## 🧪 Tests

La suite compte 28 tests (`cargo test`, exécutés sous Windows et Linux dans le CI). Ils vérifient les cas normaux, les cas limites et les entrées abîmées : fichiers vides, bruit et fichiers coupés ne font jamais paniquer. Le CI exécute aussi `cargo fmt --check`, `cargo clippy` avec les avertissements comme erreurs et `python -I tools/ci_validate.py`.

## 🏗️ Compilation et publication

La version se trouve dans `Cargo.toml` et `eh3d.project.json` ; chaque version a son entrée dans [CHANGELOG.md](CHANGELOG.md). `build.bat` / `build.sh` compilent le programme, `test.bat` / `test.sh` lancent les tests et `run.bat` / `run.sh` l'exécutent avec un exemple. Les versions sont publiées sur GitHub.

## 📂 Structure du dépôt

<p align="center">
  <img src="images/repository-structure.svg" alt="Structure du dépôt EH3D-AUTOLEVEL" width="100%">
</p>

<details>
<summary>Version texte de l'arborescence</summary>

```text
EH3D-AUTOLEVEL/
├── src/
│   ├── lib.rs                                   la bibliothèque : ce que chaque commande utilise
│   ├── main.rs                                  la ligne de commande
│   ├── cli_args.rs                              les arguments de la ligne de commande, le même fichier dans chaque petit utilitaire
│   ├── gcode.rs                                 une partie de la logique
│   └── surface.rs                               une partie de la logique
├── tests/
│   ├── autolevel.rs                             tests de la bibliothèque et de la ligne de commande
│   ├── dry_run.rs                               tests de --dry-run : compte la même chose et n'écrit rien
│   ├── readme.rs                                tests qui exécutent ce que promettent le README et les scripts run
│   └── strict.rs                                tests de la marque d'un programme nivelé et de la ligne de commande stricte
├── examples/
│   ├── sample.nc                                un exemple pour essayer le programme
│   └── table.txt                                un exemple pour essayer le programme
├── docs/
│   ├── architecture.json                        données de l'image d'Architecture
│   └── structure.json                           données de l'image de Structure
├── tools/                                       contrôle de la CI et du README et outils d'images
├── images/                                      bannière et images d'Architecture et de Structure du dépôt
├── .github/                                     workflows de CI et de publication, modèles d'issues et de pull requests
├── eh3d.project.json                            manifeste : version, rôle, maturité et commande de compilation
├── Cargo.toml Cargo.lock                        le workspace et ses dépendances verrouillées
├── CHANGELOG.md                                 ce qui a changé dans chaque version
├── SECURITY.md SUPPORT.md                       signaler une vulnérabilité et poser des questions
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   contribuer, code de conduite et licence
```

</details>

## 🗺️ Feuille de route et limites connues

**Limites connues aujourd'hui**

* Il n'a pas encore de fenêtre : seules la bibliothèque et la ligne de commande existent.
* Il n'a été essayé ni sur une machine ni avec une collection de fichiers réels, et n'a pas eu de revue indépendante.
* Les arcs sont refusés au lieu d'être coupés en morceaux, le programme est supposé commencer à X0 Y0 Z0 et la carte vient de la sonde de la suite ou d'une table simulée, pas d'une vraie mesure.

**Ensuite**

* Couper les arcs en morceaux, prendre la position de départ sur la machine et corriger le programme pendant l'envoi, pas avant.
* Ajouter la fenêtre avec l'aspect commun des programmes de la suite, en huit langues.
* L'essayer avec des fichiers réels et avec les autres programmes de la suite.

## 🌍 Langues

Le README et les textes existent en anglais, espagnol, français, italien, allemand, chinois, japonais et portugais européen.

## 🔗 Projets liés

* [EH3D-COMMON](https://github.com/JuanenRac/EH3D-COMMON): le conteneur de projet et les types partagés de la suite
* [EH3D-MOTIONCNC](https://github.com/JuanenRac/EH3D-MOTIONCNC): le programme de commande de machine
* [EH3D-ISOCAM](https://github.com/JuanenRac/EH3D-ISOCAM): le fraisage d'isolation du cuivre
* [EH3D-ROUTEPRO](https://github.com/JuanenRac/EH3D-ROUTEPRO): le programme de fraisage de contour
* [EH3D-CAM-DEBUGGER](https://github.com/JuanenRac/EH3D-CAM-DEBUGGER): la vérification du G-code avant qu'il n'atteigne une machine

## 📚 Documentation

* [Journal des modifications](CHANGELOG.md)
* [Données d'architecture et de structure](docs/architecture.json) : les données qui servent à dessiner les deux images.

## 🤝 Contribuer et communauté

* Avant une pull request, lancez `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` et `python -I tools/ci_validate.py` : tout doit passer. Voir [CONTRIBUTING.md](CONTRIBUTING.md) et le [code de conduite](CODE_OF_CONDUCT.md).
* Le README existe en huit langues : un changement de texte se fait dans les huit fichiers, et les deux images sont dessinées depuis `docs/architecture.json` et `docs/structure.json` par `tools/make_architecture.py` et `tools/make_structure.py`.
* Signalez une vulnérabilité en privé : [SECURITY.md](SECURITY.md). Questions, idées et rapports : electrohobby3d@gmail.com ([SUPPORT.md](SUPPORT.md)).

## 👤 AUTEUR

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENCE

Logiciel : GPL-3.0-or-later. Voir [LICENSE](LICENSE). Les conceptions matérielles de la suite utilisent la CERN OHL-S.
