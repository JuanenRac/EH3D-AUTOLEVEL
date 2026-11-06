<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-AUTOLEVEL banner" width="100%">
</p>

# <img src="images/brand/icon-64.png" alt="" width="40" height="40" align="absmiddle"> EH3D-AUTOLEVEL
<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | 🇮🇹 <b>Italiano</b> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### Corregge la Z di un programma G-code con la mappa delle altezze del piano

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.6-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Verifica di onestà - cosa funziona oggi:** una libreria Rust e un programma a riga di comando (`eh3d-autolevel`) con 148 test eseguiti su un computer: corregge la Z di un programma G-code con la mappa delle altezze del piano, spezzando i movimenti in tratti che seguono la superficie. Ha una finestra (compilata con `--features gui`) che apre un programma G-code e una mappa delle altezze, mostra il piano come maglia colorata con i percorsi livellati sopra ed esporta, ma la finestra non è ancora stata vista su uno schermo; non è stato provato su una macchina e non è stato verificato su una raccolta di file reali.

---

> **Stato di sicurezza:** questo programma non controlla alcuna macchina e non è un sistema di sicurezza. Ciò che produce deve essere controllato da una persona prima di eseguire qualsiasi cosa su una macchina; nulla di questo ha girato su hardware reale.


## 📋 In breve

| | |
|---|---|
| Versione | `0.0.6` |
| Maturità | Impalcatura |
| Test | 148 |
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
| Test | 🟡 | 148 test passano (`cargo test`), con casi normali, casi limite e input danneggiati. |
| Sicurezza | 🟡 | Rust sicuro e semplice (`forbid(unsafe_code)`): nulla va in panico con i file ricevuti e un valore errato è un errore con il suo motivo; nessuna revisione indipendente. |
| Finestra | 🟠 | Scritta (egui, `--features gui`): aprire o trascinare un programma e una mappa delle altezze, simulare un piano, vedere il piano come maglia colorata e i percorsi livellati, zoom e spostamento, regolare il livellamento, esportare; la sua logica è testata, ma non è ancora stata compilata né vista su uno schermo. |
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

EH3D-AUTOLEVEL è un programma della EH3D Fabrication Suite. Funziona come libreria e riga di comando e, compilato con `--features gui`, come finestra con l'aspetto comune agli altri programmi della suite. Legge e scrive file semplici, quindi può essere usato da solo o tra gli altri programmi.

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
* **Le righe che non sono movimenti passano così come sono:** i comandi `$`, le assegnazioni `#` e i messaggi `M117` e `M118` sono scritti com'erano, il checksum di una riga numerata non è una parola, le parole `M` di un movimento suddiviso vanno con l'ultimo pezzo e `--max-lines` (5000000 per impostazione predefinita) rifiuta un programma con più righe di uscita.
* **Gli assi rotanti A, B e C sono interpolati:** quando un movimento di taglio è diviso in pezzi, le sue parole `A`, `B` e `C` vanno in linea retta dal valore che avevano a quello del movimento (sono modali e partono da 0), con gli stessi decimali di X, Y e Z; l'ultimo pezzo finisce esattamente sulla destinazione e la Z del piano viene ancora solo da X e Y. Un programma senza parole rotanti è scritto come prima.
* **Il programma livellato inizia con l'intestazione standard:** il generatore e la sua versione, le unità, il programma di origine con il suo SHA-256 e gli utensili, come righe di commento prima del segno; i test rileggono il risultato con il lettore della suite e non trovano alcun avviso, alcun movimento rapido nel materiale e alcun taglio con il mandrino fermo.

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
eh3d-autolevel apply MAP.txt IN.nc OUT.nc [--segment 1.0] [--ref X,Y] [--bicubic] [--max-lines 5000000] [--decimals 3] [--config FILE] [--dry-run]
eh3d-autolevel config                            every setting with its default, as a file
eh3d-autolevel info MAP.txt                      how many points, the spread and the tilt of the table
eh3d-autolevel demo-map OUT.txt X0 Y0 X1 Y1 COLS ROWS    a map of a simulated tilted table with a bump
eh3d-autolevel [gui] [FILE...]                   the window (a build with --features gui); with no arguments it opens
```

## ⚙️ Configurazione

Un piccolo file di impostazioni (`chiave = valore`, uno per riga, `#` inizia un commento) contiene ciò che vale la pena regolare; oggi ha due chiavi: `decimals`, con quanti decimali si scrivono X, Y e Z nel programma livellato (3 per impostazione predefinita, da 1 a 6), e `max-lines`, il massimo di righe del programma livellato (5000000 per impostazione predefinita, da 1 a 100000000). `eh3d-autolevel config` stampa il file con ogni valore predefinito (`examples/autolevel.cfg` è quel file), `--config FILE` ne legge uno e `--decimals N` e `--max-lines N` danno le stesse chiavi sulla riga di comando e prevalgono sul file. Un valore senza senso è un errore che indica la sua riga. Il resto si passa come argomenti della riga di comando (vedi Interfacce) e ogni valore predefinito è scritto nella guida del comando (`eh3d-autolevel help`). I limiti fissi sono costanti con nome nel codice, coperte da test.

## 🚀 Avvio rapido

Serve una toolchain Rust stabile (1.88 o più recente).

```bash
cargo build --release
cargo test
cargo run --release -- info examples/table.txt
cargo run --release --features gui   # the window
```

## 🧪 Test

La suite ha 148 test (`cargo test`, eseguiti su Windows e Linux nella CI). Verificano i casi normali, i casi limite e gli input danneggiati: file vuoti, spazzatura e file tagliati non causano mai un panico. La CI esegue anche `cargo fmt --check`, `cargo clippy` con gli avvisi come errori e `python -I tools/ci_validate.py`.

## 🏗️ Compilazione e rilascio

La versione sta in `Cargo.toml` e in `eh3d.project.json`; ogni versione ha la sua voce in [CHANGELOG.md](CHANGELOG.md). `build.bat` / `build.sh` compilano il programma, `test.bat` / `test.sh` eseguono i test e `run.bat` / `run.sh` lo eseguono con un esempio. Le versioni sono pubblicate su GitHub.

### Download

Ogni versione contiene il programma con la sua finestra per Windows (64 e 32 bit), Linux (64 bit) e macOS (Apple silicon e Intel), come `<nome>-<versione>-<sistema>.zip` (Windows) o `.tar.gz` con il programma, la licenza, il leggimi, l'icona e gli esempi. I file sono costruiti da GitHub Actions, elencati in `SHA256SUMS` e firmati con la chiave di EH3D. Per controllare un download, confronta il suo hash con il file `.sha256` accanto ad esso (`sha256sum -c`) e verifica la firma con il file `.sig` accanto ad esso (`ssh-keygen -Y verify`).

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
│   ├── config.rs                                il file delle impostazioni: i decimali delle coordinate e il limite di righe del programma livellato
│   ├── gcode.rs                                 una parte della logica
│   ├── surface.rs                               una parte della logica
│   ├── browse.rs                                l'elenco dei file della finestra che apre un file
│   ├── gui.rs                                   la finestra (egui, con la funzione gui): disegna lo stato e inoltra ciò che fa la persona
│   ├── lang.rs                                  le parole della finestra nelle otto lingue
│   ├── viewport.rs                              zoom e spostamento dell'immagine, in semplici numeri
│   └── window.rs                                lo stato della finestra senza schermo: programma, mappa, impostazioni ed esportazione
├── tests/
│   ├── autolevel.rs                             test della libreria e della riga di comando
│   ├── decimals.rs                              test dei decimali delle coordinate e del file delle impostazioni
│   ├── dry_run.rs                               test di --dry-run: conta lo stesso e non scrive nulla
│   ├── rotary.rs                                test dell'interpolazione di A, B e C e del limite di righe del file delle impostazioni
│   ├── readme.rs                                test che eseguono ciò che promettono il README e gli script run
│   ├── strict.rs                                test del contrassegno di un programma livellato e della riga di comando rigorosa
│   └── window_cli.rs                            test che una compilazione senza la funzione gui resti una riga di comando
├── examples/
│   ├── autolevel.cfg                            il file delle impostazioni con ogni impostazione e il suo valore predefinito
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

* La finestra non è ancora stata vista su uno schermo e non può misurare un piano: la mappa si apre da un file o si simula.
* Non è stato provato su una macchina né con una raccolta di file reali e non ha avuto revisione indipendente.
* Gli archi sono rifiutati invece di essere spezzati, si presume che il programma inizi a X0 Y0 Z0 e la mappa viene dalla sonda della suite o da un piano simulato, non da una misura reale.

**Prossimi passi**

* Spezzare gli archi, prendere la posizione iniziale dalla macchina e correggere il programma durante l'invio, non prima.
* Vedere la finestra su uno schermo e finirla.
* Provarlo con file reali e con gli altri programmi della suite.

## 🌍 Lingue

Il README e i testi esistono in inglese, spagnolo, francese, italiano, tedesco, cinese, giapponese e portoghese europeo.

## 🔗 Progetti correlati

* [EH3D-COMMON](https://github.com/JuanenRac/EH3D-COMMON): il contenitore di progetto e i tipi condivisi della suite
* [EH3D-PHOENIX-CNC](https://github.com/JuanenRac/EH3D-PHOENIX-CNC): il programma di controllo della macchina
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
