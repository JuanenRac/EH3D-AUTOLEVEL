<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-AUTOLEVEL banner" width="100%">
</p>

# 📖 EH3D-AUTOLEVEL

<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | 🇮🇹 <b>Italiano</b> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### Corregge la Z di un programma G-code con la mappa delle altezze del piano

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.3-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Verifica di onestà - cosa funziona oggi:** una libreria Rust e un programma a riga di comando (`eh3d-autolevel`) con 50 test eseguiti su un computer: corregge la Z di un programma G-code con la mappa delle altezze del piano, spezzando i movimenti in tratti che seguono la superficie. Non ha ancora una finestra, non è stato provato su una macchina e non è stato verificato su una raccolta di file reali.

---

> **Stato di sicurezza:** questo programma non controlla alcuna macchina e non è un sistema di sicurezza. Ciò che produce deve essere controllato da una persona prima di eseguire qualsiasi cosa su una macchina; nulla di questo ha girato su hardware reale.


## 📋 In breve

| | |
|---|---|
| Versione | `0.0.3` |
| Maturità | Impalcatura |
| Test | 50 |
| Stack tecnologico | Rust |
| Su hardware reale | Non provato: nulla di questo ha girato su una macchina |

**Vai a:** [Panoramica](#-panoramica) · [Architettura](#-architettura) · [Avvio rapido](#-avvio-rapido) · [Test](#-test) · [Struttura](#-struttura-del-repository) · [Correlati](#-progetti-correlati)

## 🚦 Stato del progetto

Ogni riga è una parte di questo repository e il semaforo dice a che punto è oggi. È scritta in base a ciò che il repository può mostrare (test, compilazioni, esecuzioni), non a ciò che è previsto.

🟢 Fatto e verificato · 🟡 Funziona, con limiti noti · 🟠 Fatto a metà o non ancora verificato · 🔴 Non fatto o non funzionante

| Area | Stato | Cosa significa qui |
|---|---|---|
| Nucleo | 🟡 | corregge la Z di un programma G-code con la mappa delle altezze del piano, spezzando i movimenti in tratti che seguono la superficie; provato su un computer con input normali e danneggiati, non con una raccolta di file reali. |
| Riga di comando | 🟡 | `eh3d-autolevel` esegue le stesse funzioni della libreria e stampa il risultato; ogni errore torna come messaggio e codice di uscita diverso da zero. |
| Test | 🟡 | 50 test passano (`cargo test`), con casi normali, casi limite e input danneggiati. |
| Sicurezza | 🟡 | Rust sicuro e semplice (`forbid(unsafe_code)`): nulla va in panico con i file ricevuti e un valore errato è un errore con il suo motivo; nessuna revisione indipendente. |
| Finestra | 🔴 | Previsto: la finestra egui comune alla suite; oggi esiste solo la riga di comando. |
| Hardware reale | 🔴 | Non provato: nulla di questo è stato eseguito su una macchina. |
| Documentazione | 🟡 | README in otto lingue, il changelog e le due immagini disegnate da `docs/`. |

## 🎯 Panoramica

* **Il programma** (`src/gcode.rs`): un movimento di taglio (G1) è spezzato in tratti non più lunghi della lunghezza scelta e ogni tratto riceve la Z che la mappa dà per la sua fine; un movimento rapido (G0) è corretto solo se ha una Z. Lo zero della correzione è un punto di riferimento, dove è stata fissata la Z del lavoro.
* **La superficie** (`src/surface.rs`): bilineare (quella della mappa di EH3D-COMMON) o bicubica (Catmull-Rom), che passa per ogni punto misurato e segue meglio un piano curvo; fuori dalla griglia si usa l'altezza del bordo più vicino.
* **Ciò che è rifiutato**, con il numero di riga: distanze relative (G91), pollici (G20), archi (G2, G3), un movimento prima di qualsiasi G0 o G1, un numero che non lo è, una coordinata oltre un milione di millimetri, un movimento che darebbe più di centomila tratti e una mappa con punti non misurati.
* **Riga di comando rigorosa:** un'opzione scritta male (`--powr`) o una parola in più è un errore che la nomina, non qualcosa da ignorare; altrimenti il programma farebbe la sua uscita con i valori predefiniti.
* **Mai livellato due volte:** il programma corretto inizia con una riga di commento (`(eh3d-autolevel VERSIONE map=HASH ref=X,Y metodo seg=N)`) che dice che è stato livellato e con quale mappa; un programma che la porta viene rifiutato con la sua riga, perché correggerlo di nuovo sommerebbe due volte la correzione del tavolo.
* **I codici che spostano l'origine sono rifiutati:** G10, G28, G30, G38, G43, G53, G92 e ogni altro codice G che non sia un semplice movimento né un'impostazione che lascia la posizione com'è fermano il programma con il numero di riga (prima diventavano movimenti verso Z0); una pausa (G4) e la riga `%` di inizio e fine programma passano invariate, e una mappa senza altezza finita viene rifiutata invece di scrivere `Z NaN`.
* **`--dry-run`:** `apply` calcola il risultato e lo conta (movimenti, pezzi aggiunti, spostamento di Z) senza scrivere il file di uscita, che allora può essere omesso.

## 🧩 Ruolo in EH3D

EH3D-AUTOLEVEL è un programma della EH3D Fabrication Suite. Oggi funziona come libreria e riga di comando; la sua finestra, con l'aspetto comune agli altri programmi della suite, è prevista. Legge e scrive file semplici, quindi può essere usato da solo o tra gli altri programmi.

## 🏛️ Architettura

<p align="center">
  <img src="images/architecture.svg" alt="Architettura di EH3D-AUTOLEVEL: chi lo usa, che cosa contiene e da che cosa dipende" width="100%">
</p>


Come si incastrano le parti di questo repository: chi lo usa, cosa contiene e da cosa dipende. L'immagine è disegnata da `docs/architecture.json` e verificata con il codice.

## ✨ Funzionalità

* **Libreria e riga di comando:** ogni comando di `eh3d-autolevel` è una funzione della libreria, così un altro programma della suite può chiamarla senza avviare un processo.
* **Test con input danneggiati:** i test forniscono anche file vuoti, spazzatura, numeri enormi e file tagliati, e nessuno può andare in panico.
* **Nessuna ipotesi:** un valore privo di senso è un errore che dice quale e perché, mai un risultato che sembra solo giusto.
* **Nessun codice unsafe:** `#![forbid(unsafe_code)]` nella libreria e nel programma.

## 🔒 Sicurezza e protezione

| Minaccia | Difesa |
|---|---|
| Un file danneggiato, tagliato o vuoto | nessun panico: i test tagliano un input valido in ogni posizione e forniscono spazzatura |
| Un numero privo di senso (negativo, non finito, assurdamente grande) | rifiutato con un errore che nomina il valore; nulla viene arrotondato in qualcosa di plausibile |
| Uso non sicuro della memoria | Rust sicuro e semplice con `forbid(unsafe_code)` |

**Cosa non è difeso:** un risultato prodotto senza errori non è un risultato verificato. Nulla qui conosce i limiti di una macchina e il programma non ha avuto revisione indipendente. Segnalate una vulnerabilità in privato (vedi [SECURITY.md](SECURITY.md)).

## 🔌 Interfacce

La libreria è `eh3d_autolevel` (`use eh3d_autolevel::...`); la riga di comando è `eh3d-autolevel`. I comandi sono questi; ciascuno stampa il risultato sull'output standard e segnala un errore sull'output di errore con un codice di uscita diverso da zero. Non c'è accesso alla rete.

```text
EH3D-AUTOLEVEL/
├── src/
│   ├── lib.rs                                   la libreria: ciò che usa ogni comando
│   ├── main.rs                                  la riga di comando
│   ├── cli_args.rs                              gli argomenti della riga di comando, lo stesso file in ogni piccola utilità
│   ├── gcode.rs                                 una parte della logica
│   └── surface.rs                               una parte della logica
├── tests/
│   ├── autolevel.rs                             test della libreria e della riga di comando
│   ├── dry_run.rs                               test di --dry-run: conta lo stesso e non scrive nulla
│   ├── readme.rs                                test che eseguono ciò che promettono il README e gli script run
│   └── strict.rs                                test del contrassegno di un programma livellato e della riga di comando rigorosa
├── examples/
│   ├── sample.nc                                un esempio per provare il programma
│   └── table.txt                                un esempio per provare il programma
├── docs/
│   ├── architecture.json                        dati dell'immagine di Architettura
│   └── structure.json                           dati dell'immagine di Struttura
├── tools/                                       controllo della CI e del README e strumenti per le immagini
├── images/                                      banner e le immagini di Architettura e Struttura del repository
├── .github/                                     workflow di CI e di rilascio, modelli di issue e pull request
├── eh3d.project.json                            manifesto: versione, ruolo, maturità e comando di compilazione
├── Cargo.toml Cargo.lock                        il workspace e le sue dipendenze bloccate
├── rust-toolchain.toml                          la toolchain Rust fissata
├── CHANGELOG.md                                 cosa è cambiato in ogni versione
├── SECURITY.md SUPPORT.md                       come segnalare una vulnerabilità e dove chiedere
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   come contribuire, codice di condotta e licenza
```

## ⚙️ Configurazione

Non c'è ancora un file di configurazione: ciò che la persona sceglie si passa come argomenti della riga di comando (vedi Interfacce) e ogni valore predefinito è scritto nella guida del comando (`eh3d-autolevel help`). I limiti fissi sono costanti con nome nel codice, coperte da test.

## 🚀 Avvio rapido

Serve una toolchain Rust stabile (1.75 o più recente).

```bash
cargo build --release
cargo test
cargo run --release -- info examples/table.txt
```

## 🧪 Test

La suite ha 50 test (`cargo test`, eseguiti su Windows e Linux nella CI). Verificano i casi normali, i casi limite e gli input danneggiati: file vuoti, spazzatura e file tagliati non causano mai un panico. La CI esegue anche `cargo fmt --check`, `cargo clippy` con gli avvisi come errori e `python -I tools/ci_validate.py`.

## 🏗️ Compilazione e rilascio

La versione sta in `Cargo.toml` e in `eh3d.project.json`; ogni versione ha la sua voce in [CHANGELOG.md](CHANGELOG.md). `build.bat` / `build.sh` compilano il programma, `test.bat` / `test.sh` eseguono i test e `run.bat` / `run.sh` lo eseguono con un esempio. Le versioni sono pubblicate su GitHub.

## 📂 Struttura del repository

<p align="center">
  <img src="images/repository-structure.svg" alt="Struttura del repository EH3D-AUTOLEVEL" width="100%">
</p>

<details>
<summary>Versione testuale dell'albero</summary>

```text
EH3D-AUTOLEVEL/
├── src/
│   ├── lib.rs                                   la libreria: ciò che usa ogni comando
│   ├── main.rs                                  la riga di comando
│   ├── cli_args.rs                              gli argomenti della riga di comando, lo stesso file in ogni piccola utilità
│   ├── gcode.rs                                 una parte della logica
│   └── surface.rs                               una parte della logica
├── tests/
│   ├── autolevel.rs                             test della libreria e della riga di comando
│   ├── dry_run.rs                               test di --dry-run: conta lo stesso e non scrive nulla
│   ├── readme.rs                                test che eseguono ciò che promettono il README e gli script run
│   └── strict.rs                                test del contrassegno di un programma livellato e della riga di comando rigorosa
├── examples/
│   ├── sample.nc                                un esempio per provare il programma
│   └── table.txt                                un esempio per provare il programma
├── docs/
│   ├── architecture.json                        dati dell'immagine di Architettura
│   └── structure.json                           dati dell'immagine di Struttura
├── tools/                                       controllo della CI e del README e strumenti per le immagini
├── images/                                      banner e le immagini di Architettura e Struttura del repository
├── .github/                                     workflow di CI e di rilascio, modelli di issue e pull request
├── eh3d.project.json                            manifesto: versione, ruolo, maturità e comando di compilazione
├── Cargo.toml Cargo.lock                        il workspace e le sue dipendenze bloccate
├── rust-toolchain.toml                          la toolchain Rust fissata
├── CHANGELOG.md                                 cosa è cambiato in ogni versione
├── SECURITY.md SUPPORT.md                       come segnalare una vulnerabilità e dove chiedere
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   come contribuire, codice di condotta e licenza
```

</details>

## 🗺️ Roadmap e limiti noti

**Limiti noti oggi**

* Non ha ancora una finestra: esistono solo la libreria e la riga di comando.
* Non è stato provato su una macchina né con una raccolta di file reali e non ha avuto revisione indipendente.
* Gli archi sono rifiutati invece di essere spezzati, si presume che il programma inizi a X0 Y0 Z0 e la mappa viene dalla sonda della suite o da un piano simulato, non da una misura reale.

**Prossimi passi**

* Spezzare gli archi, prendere la posizione iniziale dalla macchina e correggere il programma durante l'invio, non prima.
* Aggiungere la finestra con l'aspetto comune ai programmi della suite, negli otto idiomi.
* Provarlo con file reali e con gli altri programmi della suite.

## 🌍 Lingue

Il README e i testi esistono in inglese, spagnolo, francese, italiano, tedesco, cinese, giapponese e portoghese europeo.

## 🔗 Progetti correlati

* [EH3D-COMMON](https://github.com/JuanenRac/EH3D-COMMON): il contenitore di progetto e i tipi condivisi della suite
* [EH3D-MOTIONCNC](https://github.com/JuanenRac/EH3D-MOTIONCNC): il programma di controllo della macchina
* [EH3D-ISOCAM](https://github.com/JuanenRac/EH3D-ISOCAM): la fresatura di isolamento del rame
* [EH3D-ROUTEPRO](https://github.com/JuanenRac/EH3D-ROUTEPRO): il programma di fresatura del contorno
* [EH3D-CAM-DEBUGGER](https://github.com/JuanenRac/EH3D-CAM-DEBUGGER): il controllo del G-code prima che arrivi a una macchina

## 📚 Documentazione

* [Changelog](CHANGELOG.md)
* [Dati di architettura e struttura](docs/architecture.json): i dati da cui si disegnano le due immagini.

## 🤝 Contribuire e comunità

* Prima di una pull request eseguite `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` e `python -I tools/ci_validate.py`: tutto deve passare. Vedi [CONTRIBUTING.md](CONTRIBUTING.md) e il [codice di condotta](CODE_OF_CONDUCT.md).
* Il README esiste in otto lingue: una modifica al testo si fa in tutti e otto i file, e le due immagini sono disegnate da `docs/architecture.json` e `docs/structure.json` con `tools/make_architecture.py` e `tools/make_structure.py`.
* Segnalate una vulnerabilità in privato: [SECURITY.md](SECURITY.md). Domande, idee e segnalazioni: electrohobby3d@gmail.com ([SUPPORT.md](SUPPORT.md)).

## 👤 AUTORE

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENZA

Software: GPL-3.0-or-later. Vedi [LICENSE](LICENSE). I progetti hardware della suite usano la CERN OHL-S.
