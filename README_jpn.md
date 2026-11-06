<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-AUTOLEVEL banner" width="100%">
</p>

# <img src="images/brand/icon-64.png" alt="" width="40" height="40" align="absmiddle"> EH3D-AUTOLEVEL
<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | 🇯🇵 <b>日本語</b> | <a href="README_por.md">🇵🇹 Português</a></p>

### テーブルの高さマップで G コード プログラムの Z を補正

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.7-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**正直さの確認 - 現在実際に動くもの：** Rust ライブラリとコマンドライン プログラム（`eh3d-autolevel`）で、コンピューター上で実行される 172 件のテストがあります：テーブルの高さマップで G コード プログラムの Z を補正し、移動を表面に沿った短い区間に分割します。ウィンドウ（`--features gui` でビルド）はG コード プログラムと高さマップを開き、テーブルを色付きメッシュで表示してその上に補正後のパスを重ね、書き出すが、画面ではまだ確認しておらず、機械では試しておらず、実際のファイルの集まりでも確認していません。

---

> **安全とセキュリティの状況：** このプログラムは機械を制御せず、安全システムでもありません。出力は、機械で何かを実行する前に人が確認する必要があります。ここにあるものは実機で動かしたことがありません。


## 📋 ひと目で

| | |
|---|---|
| バージョン | `0.0.7` |
| 成熟度 | 足場 |
| テスト | 172 |
| 技術スタック | Rust |
| 実機での動作 | 未試験：ここにあるものは機械で動かしたことがありません |

**移動先：** [概要](#-概要) · [アーキテクチャ](#-アーキテクチャ) · [クイックスタート](#-クイックスタート) · [テスト](#-テスト) · [構成](#-リポジトリの構成) · [関連](#-関連プロジェクト)

## 🚦 プロジェクトの状況

各行はこのリポジトリの一部分で、信号は今日どこまで進んでいるかを示します。計画ではなく、リポジトリが示せるもの（テスト、ビルド、実行）に基づいて書かれています。

🟢 完了・検証済み · 🟡 動作する（既知の制限あり） · 🟠 一部のみ、または未検証 · 🔴 未実装または動作しない

| 領域 | 状態 | ここでの意味 |
|---|---|---|
| コア | 🟡 | テーブルの高さマップで G コード プログラムの Z を補正し、移動を表面に沿った短い区間に分割します。コンピューター上で通常入力と破損入力でテスト済み。実際のファイルの集まりでは未確認。 |
| コマンドライン | 🟡 | `eh3d-autolevel` はライブラリと同じ関数を実行して結果を表示します。エラーはメッセージと 0 以外の終了コードで返ります。 |
| テスト | 🟡 | 172 件のテストが通ります（`cargo test`）。通常ケース、境界ケース、破損入力を含みます。 |
| セキュリティ | 🟡 | 安全な Rust のみ（`forbid(unsafe_code)`）。受け取ったファイルでパニックせず、不正な値は理由付きのエラーになります。独立したレビューはありません。 |
| ウィンドウ | 🟠 | 作成済み（egui、`--features gui`）：プログラムと高さマップを開くかドロップ、テーブルのシミュレート、色付きメッシュのテーブルと補正後のパスの表示、拡大縮小と移動、補正の設定、書き出し。ロジックはテスト済みですが、まだビルドも画面での確認もしていません。 |
| 実機 | 🔴 | 未試験：ここにあるものは機械で実行したことがありません。 |
| ドキュメント | 🟡 | 8 言語の README、変更履歴、`docs/` から描いた 2 枚の図。 |

## 🎯 概要

* **プログラム**（`src/gcode.rs`）：切削移動（G1）は指定した長さ以下の区間に分割され、各区間の終点にマップが与える Z が付きます。早送り（G0）は Z がある場合のみ補正します。補正のゼロは、ジョブの Z を設定した基準点です。
* **表面**（`src/surface.rs`）：双線形（EH3D-COMMON のマップのもの）または双三次（Catmull-Rom）。双三次はすべての測定点を通り、湾曲したテーブルにより良く追従します。グリッドの外では最も近い端の高さを使います。
* **拒否するもの**（行番号付き）：相対距離（G91）、インチ（G20）、円弧（G2、G3）、G0 や G1 より前の移動、数値でない数値、100 万 mm を超える座標、10 万区間を超える移動、未測定の点があるマップ。
* **厳密なコマンドライン：** 書き間違えたオプション（`--powr`）や余分な語は無視されず、それを名指しするエラーになります。そうでないと、プログラムは既定値で出力を作ってしまいます。
* **二度は補正しない：** 補正後のプログラムは、補正済みであることと使ったマップを示すコメント行（`(eh3d-autolevel VERSION map=HASH ref=X,Y 方式 seg=N)`）で始まります。この行を持つプログラムは行番号付きで拒否されます。もう一度補正するとテーブルの補正が二重に加わるためです。
* **原点を動かすコードは拒否されます：** G10、G28、G30、G38、G43、G53、G92、および単純な移動でも位置を変えない設定でもない G コードは、行番号を示してプログラムを止めます（以前は Z0 への移動に変換されていました）。ドウェル（G4）とプログラムの先頭・末尾の `%` 行はそのまま通り、有限の高さを返さないマップは `Z NaN` を書かずに拒否されます。
* **`--dry-run`：** `apply` は結果を計算して集計し（移動数、追加された区間、Zのずれ）、出力ファイルは書き出しません（その場合は省略できます）。

## 🧩 EH3D での役割

EH3D-AUTOLEVEL は EH3D Fabrication Suite のプログラムの一つです。ライブラリとコマンドラインとして動作し、`--features gui` でビルドすると、スイートの他のプログラムと共通の外観のウィンドウとしても動作します。単純なファイルを読み書きするため、単独でも他のプログラムの間でも使えます。

## 🏛️ アーキテクチャ

<p align="center">
  <img src="images/architecture.svg" alt="EH3D-AUTOLEVEL のアーキテクチャ：誰が使い、何を持ち、何に依存するか" width="100%">
</p>


このリポジトリの各部分の関係：誰が使い、何を持ち、何に依存するか。図は `docs/architecture.json` から描かれ、コードと照合されます。

## ✨ 機能

* **ライブラリとコマンドライン：** `eh3d-autolevel` の各コマンドはライブラリの関数なので、スイートの他のプログラムがプロセスを起動せずに呼び出せます。
* **破損入力のテスト：** 空のファイル、ゴミデータ、巨大な数値、途中で切れたファイルも与え、どれもパニックしてはいけません。
* **推測しない：** 意味のない値は、どの値かと理由を示すエラーになり、見かけだけ正しい結果にはなりません。
* **unsafe コードなし：** ライブラリとプログラムに `#![forbid(unsafe_code)]`。
* **移動でない行はそのまま通します：** `$` コマンド、`#` の代入、メッセージ `M117` と `M118` はそのまま書き出し、番号付き行のチェックサムは語として扱わず、分割した移動の `M` 語は最後の断片に付け、`--max-lines`（既定は 5000000）は出力行数がそれを超えるプログラムを拒否します。
* **回転軸 A、B、C を補間します：** 切削移動を断片に分割するとき、`A`、`B`、`C` の語は元の値から移動の指定値まで直線で変化し（モーダルで、初期値は0）、X、Y、Z と同じ小数桁数で書かれます。最後の断片は目標値にぴったり一致し、テーブルの Z は引き続き X と Y だけから求めます。回転軸の語がないプログラムはこれまでどおり書き出されます。
* **A 軸の回転による Z 補正：** `a-axis-compensation = tilt` と `a-axis-radius-mm = r`（または `--a-compensation tilt --a-radius r`）を指定すると、書き出す各移動の Z に、テーブルの高さの後で `r * (1 - cos A)` が加算されます。A は移動が残す角度（度）で、片ごとに補間されます。既定は `off` で、出力は一切変わらず、適用されたときはエラー出力に通知が表示されます。式は `docs/a-axis-compensation.md` にあります。
* **レベリング後のプログラムは標準ヘッダで始まります:** 生成元とそのバージョン、単位、元のプログラムとそのSHA-256、工具が、印の前のコメント行として入ります。テストは結果をスイートのリーダーで読み直し、警告、材料への早送り、スピンドルが止まったままの切削が1つも見つからないことを確かめます。

## 🔒 セキュリティと安全

| 脅威 | 対策 |
|---|---|
| 破損・途中切れ・空のファイル | パニックなし：テストは有効な入力をあらゆる位置で切り、ゴミデータも与えます |
| 意味のない数値（負、無限、極端に大きい） | 値を示すエラーで拒否され、もっともらしい値に丸められることはありません |
| 安全でないメモリ使用 | `forbid(unsafe_code)` の安全な Rust のみ |

**防御していないこと：** エラーなく作られた結果は、検証された結果ではありません。ここには機械の限界を知るものはなく、プログラムは独立したレビューを受けていません。脆弱性は非公開で報告してください（[SECURITY.md](SECURITY.md) 参照）。

## 🔌 インターフェース

ライブラリは `eh3d_autolevel`（`use eh3d_autolevel::...`）、コマンドラインは `eh3d-autolevel` です。コマンドは次のとおりで、結果は標準出力に表示され、エラーは標準エラー出力に 0 以外の終了コードで報告されます。ネットワークアクセスはありません。

```text
eh3d-autolevel apply MAP.txt IN.nc OUT.nc [--segment 1.0] [--ref X,Y] [--bicubic] [--max-lines 5000000] [--decimals 3] [--warn-shift MM] [--a-compensation off|tilt] [--a-radius MM] [--config FILE] [--dry-run]
eh3d-autolevel config                            every setting with its default, as a file
eh3d-autolevel info MAP.txt                      how many points, the spread and the tilt of the table
eh3d-autolevel demo-map OUT.txt X0 Y0 X1 Y1 COLS ROWS    a map of a simulated tilted table with a bump
eh3d-autolevel [gui] [FILE...]                   the window (a build with --features gui); with no arguments it opens
```

## ⚙️ 設定

調整する価値のある値は小さな設定ファイル（`キー = 値`、1行に1つ、`#` からコメント）に置きます。現在のキーは5つあります。`decimals` は補正後プログラムの X、Y、Z を何桁の小数で書くか（既定は3、1〜6）、`max-lines` は補正後プログラムの最大行数（既定は 5000000、1〜100000000）を決めます。`eh3d-autolevel config` はすべての既定値を含むファイルを出力し（`examples/autolevel.cfg` がそのファイルです）、`--config FILE` はファイルを読み、`--decimals N` と `--max-lines N` は同じキーをコマンドラインで与えてファイルより優先されます。意味のない値は、その行番号を示すエラーになります。それ以外はコマンドラインの引数で渡し（インターフェースを参照）、各既定値はコマンドのヘルプ（`eh3d-autolevel help`）に書かれています。固定の上限はコード内の名前付き定数で、テストで確認されています。 `warn-shift-mm` は、これを超える Z 補正があると `apply` がエラー出力に警告を表示する値（mm）です（既定は 0 で警告なし、0.001〜100）。コマンドラインの `--warn-shift MM` が設定ファイルより優先され、補正後のプログラムはそのまま書き出されます。`a-axis-compensation`（`off` または `tilt`、既定は `off`、オプションは `--a-compensation`）と `a-axis-radius-mm`（A 軸から切削点までの距離 mm、-1000〜1000 で、`tilt` では 0 以外、オプションは `--a-radius`）は、A 軸の回転で生じる補正を Z に加えます（`docs/a-axis-compensation.md` を参照）。オプションは設定ファイルより優先されます。

## 🚀 クイックスタート

安定版の Rust ツールチェーン（1.88 以降）が必要です。

```bash
cargo build --release
cargo test
cargo run --release -- info examples/table.txt
cargo run --release --features gui   # the window
cargo run --release --features gui -- --shot screen.png examples/sample.nc   # a picture of the window
```

手動で開かずにウィンドウの画像を撮るには、`--shot FILE.png` を付けて実行します（`--size WxH`, `--theme forge|dark|light`, `--wait N`, `--lang CODE`, `--open FILE`, `--replace` も使えます）。指定のサイズで開き、PNG を保存し、設定は保持せず、自動で閉じます（終了コードは、ファイルを書き込めたら 0、書けなければ 1、引数が誤りなら 2）。

## 🧪 テスト

テストは 172 件あります（`cargo test`、CI では Windows と Linux で実行）。通常ケース、境界ケース、破損入力を確認し、空のファイル、ゴミデータ、途中で切れたファイルでもパニックしません。CI は `cargo fmt --check`、警告をエラーにした `cargo clippy`、`python -I tools/ci_validate.py` も実行します。

## 🏗️ ビルドとリリース

バージョンは `Cargo.toml` と `eh3d.project.json` にあり、各バージョンは [CHANGELOG.md](CHANGELOG.md) に項目があります。`build.bat` / `build.sh` でビルド、`test.bat` / `test.sh` でテスト、`run.bat` / `run.sh` でサンプル実行します。リリースは GitHub で公開されます。

### ダウンロード

各バージョンには、Windows（64 ビットと 32 ビット）、Linux（64 ビット）、macOS（Apple silicon と Intel）向けの、ウィンドウ付きのプログラムが `<名前>-<バージョン>-<システム>.zip`（Windows）または `.tar.gz` として含まれ、プログラム、ライセンス、README、アイコン、サンプルが入っています。ファイルは GitHub Actions でビルドされ、`SHA256SUMS` に記載され、EH3D の鍵で署名されています。ダウンロードを確認するには、隣にある `.sha256` ファイルとハッシュを比較し（`sha256sum -c`）、隣にある `.sig` ファイルで署名を確認します（`ssh-keygen -Y verify`）。

## 📂 リポジトリの構成

<p align="center">
  <img src="images/repository-structure.svg" alt="EH3D-AUTOLEVEL のリポジトリ構成" width="100%">
</p>

<details>
<summary>ツリーのテキスト版</summary>

```text
EH3D-AUTOLEVEL/
├── src/
│   ├── lib.rs                                   ライブラリ：各コマンドが使うもの
│   ├── main.rs                                  コマンドライン
│   ├── cli_args.rs                              コマンドライン引数：小さなツールすべてで同じファイル
│   ├── config.rs                                設定ファイル：補正後プログラムの座標の小数桁数と行数の上限
│   ├── gcode.rs                                 ロジックの一部
│   ├── surface.rs                               ロジックの一部
│   ├── browse.rs                                ファイルを開くウィンドウのファイル一覧
│   ├── gui.rs                                   ウィンドウ（egui、gui 機能）：状態を描き、操作を伝える
│   ├── lang.rs                                  ウィンドウの文言（8言語）
│   ├── shot.rs                                  ウィンドウの画像モード（--shot）：画面なしで読み取り・検証される引数
│   ├── viewport.rs                              画像の拡大縮小と移動（単純な数値）
│   └── window.rs                                画面なしのウィンドウの状態：プログラム、マップ、設定、書き出し
├── tests/
│   ├── a_axis.rs                                A 軸の回転による Z 補正、その設定キーとオプションのテスト
│   ├── autolevel.rs                             ライブラリとコマンドラインのテスト
│   ├── decimals.rs                              座標の小数桁数と設定ファイルのテスト
│   ├── dry_run.rs                               --dry-runのテスト：同じ数を数え、何も書かない
│   ├── rotary.rs                                A、B、C の補間と設定ファイルの行数上限のテスト
│   ├── readme.rs                                READMEとrunスクリプトが約束する内容を実行するテスト
│   ├── strict.rs                                補正済みプログラムの印と厳密なコマンドラインのテスト
│   ├── window_cli.rs                            gui 機能なしのビルドがコマンドラインのままであることのテスト
│   └── warn_shift.rs                            上限を超える Z 補正の警告とその設定キーのテスト
├── examples/
│   ├── autolevel.cfg                            すべての設定と既定値を載せた設定ファイル
│   ├── sample.nc                                試すためのサンプル
│   └── table.txt                                試すためのサンプル
├── docs/
│   ├── a-axis-compensation.md                   A 軸の回転による Z 補正の式と計算例
│   ├── architecture.json                        アーキテクチャ図のデータ
│   └── structure.json                           構成図のデータ
├── tools/                                       CI と README のチェック、図のツール
├── images/                                      バナーとアーキテクチャ・リポジトリ構成の図
├── .github/                                     CI とリリースのワークフロー、Issue と PR のテンプレート
├── eh3d.project.json                            マニフェスト：バージョン、役割、成熟度、ビルドコマンド
├── Cargo.toml Cargo.lock                        ワークスペースと固定された依存関係
├── rust-toolchain.toml                          固定されたRustツールチェーン
├── CHANGELOG.md                                 各バージョンの変更点
├── SECURITY.md SUPPORT.md                       脆弱性の報告方法と問い合わせ先
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   貢献方法、行動規範、ライセンス
```

</details>

## 🗺️ ロードマップと既知の制限

**現時点で分かっている制限**

* ウィンドウはまだ画面で確認しておらず、テーブルを測定することもできません。マップはファイルから開くかシミュレートします。
* 機械でも実際のファイルの集まりでも試しておらず、独立したレビューも受けていません。
* 円弧は分割せず拒否し、プログラムは X0 Y0 Z0 から始まると仮定し、マップはスイートのプローブまたは模擬テーブルからで、実測ではありません。

**次のステップ**

* 円弧を分割し、開始位置を機械から取得し、事前ではなく送信中にプログラムを補正する。
* ウィンドウを画面で確認して仕上げる。
* 実際のファイルとスイートの他のプログラムで試す。

## 🌍 言語

README とテキストは、英語、スペイン語、フランス語、イタリア語、ドイツ語、中国語、日本語、ヨーロッパポルトガル語で提供されています。

## 🔗 関連プロジェクト

* [EH3D-COMMON](https://github.com/JuanenRac/EH3D-COMMON): プロジェクト コンテナーとスイート共通の型
* [EH3D-PHOENIX-CNC](https://github.com/JuanenRac/EH3D-PHOENIX-CNC): 機械制御プログラム
* [EH3D-ISOCAM](https://github.com/JuanenRac/EH3D-ISOCAM): 銅箔の絶縁ミリング
* [EH3D-ROUTEPRO](https://github.com/JuanenRac/EH3D-ROUTEPRO): 輪郭ミリング プログラム
* [EH3D-CAM-DEBUGGER](https://github.com/JuanenRac/EH3D-CAM-DEBUGGER): 機械に届く前の G コード検査

## 📚 ドキュメント

* [変更履歴](CHANGELOG.md)
* [アーキテクチャと構成のデータ](docs/architecture.json)：2 枚の図を描くためのデータ。

## 🤝 コントリビュートとコミュニティ

* プルリクエストの前に `cargo fmt --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test`、`python -I tools/ci_validate.py` を実行し、すべて通してください。[CONTRIBUTING.md](CONTRIBUTING.md) と[行動規範](CODE_OF_CONDUCT.md)を参照。
* README は 8 言語あります。文章の変更は 8 ファイルすべてで行い、2 枚の図は `tools/make_architecture.py` と `tools/make_structure.py` が `docs/architecture.json` と `docs/structure.json` から描きます。
* 脆弱性は非公開で報告してください：[SECURITY.md](SECURITY.md)。質問・提案・報告：electrohobby3d@gmail.com（[SUPPORT.md](SUPPORT.md)）。

## 👤 作者

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 ライセンス

ソフトウェア：GPL-3.0-or-later。[LICENSE](LICENSE) を参照。スイートのハードウェア設計には CERN OHL-S を使います。
