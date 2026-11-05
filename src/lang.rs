//! The words of the window in the eight languages of the repository (English, Spanish, German, French, Italian, Japanese, Chinese and Portuguese of Portugal).

/// The languages of the window, in the order the lists of words below follow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    Es,
    De,
    Fr,
    It,
    Ja,
    Zh,
    Pt,
}

impl Lang {
    pub const ALL: [Lang; 8] = [
        Lang::En,
        Lang::Es,
        Lang::De,
        Lang::Fr,
        Lang::It,
        Lang::Ja,
        Lang::Zh,
        Lang::Pt,
    ];

    /// The two letters of the language (`en`, `es`, `de`, `fr`, `it`, `ja`, `zh`, `pt`).
    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Es => "es",
            Lang::De => "de",
            Lang::Fr => "fr",
            Lang::It => "it",
            Lang::Ja => "ja",
            Lang::Zh => "zh",
            Lang::Pt => "pt",
        }
    }

    /// The name of the language in the language itself.
    pub fn name(self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::Es => "Español",
            Lang::De => "Deutsch",
            Lang::Fr => "Français",
            Lang::It => "Italiano",
            Lang::Ja => "日本語",
            Lang::Zh => "中文",
            Lang::Pt => "Português",
        }
    }

    /// The language of a code such as `es`, `es_ES.UTF-8` or `pt-PT`.
    pub fn from_code(code: &str) -> Option<Lang> {
        let two: String = code
            .chars()
            .take(2)
            .collect::<String>()
            .to_ascii_lowercase();
        Lang::ALL.into_iter().find(|l| l.code() == two)
    }

    /// The language of the person from the variables of the system; English when it is none of the eight.
    pub fn from_env() -> Lang {
        for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
            if let Ok(v) = std::env::var(var) {
                if !v.is_empty() {
                    return Lang::from_code(&v).unwrap_or(Lang::En);
                }
            }
        }
        Lang::En
    }
}

/// Puts the values in the places `{0}`, `{1}`... of a text.
pub fn fill(template: &str, values: &[&str]) -> String {
    let mut out = template.to_string();
    for (k, v) in values.iter().enumerate() {
        out = out.replace(&format!("{{{k}}}"), v);
    }
    out
}

/// Every word and sentence of the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Msg {
    Language,
    Fit,
    Cancel,
    Up,
    AllFiles,
    EmptyFolder,
    FileName,
    Export,
    ZoomHint,
    NoFile,
    Ready,
    Failed,
    Written,
    SameFile,
    BadValue,
    CursorPlain,
    Stale,
    OpenProgram,
    OpenMap,
    Simulate,
    Compute,
    ExportProgram,
    ExportMap,
    TitleOpenProgram,
    TitleOpenMap,
    TitleExportProgram,
    TitleExportMap,
    SectionFiles,
    SectionParameters,
    SectionSimulation,
    SectionMap,
    SectionLayers,
    Segment,
    RefX,
    RefY,
    MaxLines,
    Method,
    Bilinear,
    Bicubic,
    SimX0,
    SimY0,
    SimX1,
    SimY1,
    SimCols,
    SimRows,
    MeasureHint,
    ShowMap,
    ShowOriginal,
    ShowCorrected,
    LegendLow,
    LegendHigh,
    LegendTable,
    LegendOriginal,
    LegendCorrected,
    DropHint,
    InfoPoints,
    InfoSpread,
    InfoTilt,
    ProgramOpened,
    MapOpened,
    Simulated,
    Computed,
    NeedProgram,
    NeedMap,
    Truncated,
    CursorTable,
    SimulatedName,
    BadFile,
}

impl Msg {
    pub const ALL: [Msg; 68] = [
        Msg::Language,
        Msg::Fit,
        Msg::Cancel,
        Msg::Up,
        Msg::AllFiles,
        Msg::EmptyFolder,
        Msg::FileName,
        Msg::Export,
        Msg::ZoomHint,
        Msg::NoFile,
        Msg::Ready,
        Msg::Failed,
        Msg::Written,
        Msg::SameFile,
        Msg::BadValue,
        Msg::CursorPlain,
        Msg::Stale,
        Msg::OpenProgram,
        Msg::OpenMap,
        Msg::Simulate,
        Msg::Compute,
        Msg::ExportProgram,
        Msg::ExportMap,
        Msg::TitleOpenProgram,
        Msg::TitleOpenMap,
        Msg::TitleExportProgram,
        Msg::TitleExportMap,
        Msg::SectionFiles,
        Msg::SectionParameters,
        Msg::SectionSimulation,
        Msg::SectionMap,
        Msg::SectionLayers,
        Msg::Segment,
        Msg::RefX,
        Msg::RefY,
        Msg::MaxLines,
        Msg::Method,
        Msg::Bilinear,
        Msg::Bicubic,
        Msg::SimX0,
        Msg::SimY0,
        Msg::SimX1,
        Msg::SimY1,
        Msg::SimCols,
        Msg::SimRows,
        Msg::MeasureHint,
        Msg::ShowMap,
        Msg::ShowOriginal,
        Msg::ShowCorrected,
        Msg::LegendLow,
        Msg::LegendHigh,
        Msg::LegendTable,
        Msg::LegendOriginal,
        Msg::LegendCorrected,
        Msg::DropHint,
        Msg::InfoPoints,
        Msg::InfoSpread,
        Msg::InfoTilt,
        Msg::ProgramOpened,
        Msg::MapOpened,
        Msg::Simulated,
        Msg::Computed,
        Msg::NeedProgram,
        Msg::NeedMap,
        Msg::Truncated,
        Msg::CursorTable,
        Msg::SimulatedName,
        Msg::BadFile,
    ];

    /// The words in the order English, Spanish, German, French, Italian, Japanese, Chinese, Portuguese.
    fn words(self) -> [&'static str; 8] {
        match self {
            Msg::Language => [
                "Language",
                "Idioma",
                "Sprache",
                "Langue",
                "Lingua",
                "言語",
                "语言",
                "Idioma",
            ],
            Msg::Fit => [
                "Fit to window",
                "Ajustar a la ventana",
                "An Fenster anpassen",
                "Ajuster à la fenêtre",
                "Adatta alla finestra",
                "ウィンドウに合わせる",
                "适应窗口",
                "Ajustar à janela",
            ],
            Msg::Cancel => [
                "Cancel",
                "Cancelar",
                "Abbrechen",
                "Annuler",
                "Annulla",
                "キャンセル",
                "取消",
                "Cancelar",
            ],
            Msg::Up => [
                "Up",
                "Subir",
                "Hoch",
                "Dossier parent",
                "Su",
                "上へ",
                "上一级",
                "Subir",
            ],
            Msg::AllFiles => [
                "All files",
                "Todos los ficheros",
                "Alle Dateien",
                "Tous les fichiers",
                "Tutti i file",
                "すべてのファイル",
                "所有文件",
                "Todos os ficheiros",
            ],
            Msg::EmptyFolder => [
                "No files of this kind here",
                "No hay ficheros de este tipo aquí",
                "Hier gibt es keine Dateien dieser Art",
                "Aucun fichier de ce type ici",
                "Nessun file di questo tipo qui",
                "この種類のファイルはありません",
                "此处没有这类文件",
                "Não há ficheiros deste tipo aqui",
            ],
            Msg::FileName => [
                "File name",
                "Nombre del fichero",
                "Dateiname",
                "Nom du fichier",
                "Nome del file",
                "ファイル名",
                "文件名",
                "Nome do ficheiro",
            ],
            Msg::Export => [
                "Export",
                "Exportar",
                "Exportieren",
                "Exporter",
                "Esporta",
                "書き出し",
                "导出",
                "Exportar",
            ],
            Msg::ZoomHint => [
                "Wheel: zoom - drag: move - double click: fit",
                "Rueda: zoom - arrastrar: mover - doble clic: ajustar",
                "Rad: Zoom - Ziehen: verschieben - Doppelklick: anpassen",
                "Molette : zoom - glisser : déplacer - double clic : ajuster",
                "Rotella: zoom - trascina: sposta - doppio clic: adatta",
                "ホイール: ズーム - ドラッグ: 移動 - ダブルクリック: 全体表示",
                "滚轮：缩放 - 拖动：平移 - 双击：适应窗口",
                "Roda: zoom - arrastar: mover - duplo clique: ajustar",
            ],
            Msg::NoFile => [
                "(no file)",
                "(sin fichero)",
                "(keine Datei)",
                "(aucun fichier)",
                "(nessun file)",
                "（ファイルなし）",
                "（无文件）",
                "(sem ficheiro)",
            ],
            Msg::Ready => [
                "Ready",
                "Listo",
                "Bereit",
                "Prêt",
                "Pronto",
                "準備完了",
                "就绪",
                "Pronto",
            ],
            Msg::Failed => [
                "{0}",
                "{0}",
                "{0}",
                "{0}",
                "{0}",
                "{0}",
                "{0}",
                "{0}",
            ],
            Msg::Written => [
                "Written: {0}",
                "Escrito: {0}",
                "Geschrieben: {0}",
                "Écrit : {0}",
                "Scritto: {0}",
                "書き込みました: {0}",
                "已写入：{0}",
                "Escrito: {0}",
            ],
            Msg::SameFile => [
                "{0}: the output file is also an input file; write to another file",
                "{0}: el fichero de salida es también un fichero de entrada; escribe en otro",
                "{0}: Die Ausgabedatei ist auch eine Eingabedatei; schreiben Sie in eine andere Datei",
                "{0} : le fichier de sortie est aussi un fichier d'entrée ; écrivez dans un autre fichier",
                "{0}: il file di uscita è anche un file di ingresso; scrivi su un altro file",
                "{0}: 出力ファイルが入力ファイルと同じです。別のファイルに書き込んでください",
                "{0}：输出文件同时也是输入文件，请写入另一个文件",
                "{0}: o ficheiro de saída é também um ficheiro de entrada; escreva noutro ficheiro",
            ],
            Msg::BadValue => [
                "{0}: this value is not valid",
                "{0}: este valor no es válido",
                "{0}: Dieser Wert ist ungültig",
                "{0} : cette valeur n'est pas valide",
                "{0}: questo valore non è valido",
                "{0}: この値は無効です",
                "{0}：该值无效",
                "{0}: este valor não é válido",
            ],
            Msg::CursorPlain => [
                "X {0}  Y {1}",
                "X {0}  Y {1}",
                "X {0}  Y {1}",
                "X {0}  Y {1}",
                "X {0}  Y {1}",
                "X {0}  Y {1}",
                "X {0}  Y {1}",
                "X {0}  Y {1}",
            ],
            Msg::Stale => [
                "The settings changed: level the program again",
                "Los ajustes cambiaron: vuelve a nivelar el programa",
                "Die Einstellungen haben sich geändert: Programm erneut nivellieren",
                "Les réglages ont changé : nivelez à nouveau le programme",
                "Le impostazioni sono cambiate: livella di nuovo il programma",
                "設定が変わりました。もう一度補正してください",
                "设置已更改：请重新校平程序",
                "As definições mudaram: nivele o programa outra vez",
            ],
            Msg::OpenProgram => [
                "Open G-code...",
                "Abrir G-code...",
                "G-Code öffnen...",
                "Ouvrir un G-code...",
                "Apri G-code...",
                "G-code を開く...",
                "打开 G 代码...",
                "Abrir G-code...",
            ],
            Msg::OpenMap => [
                "Open height map...",
                "Abrir mapa de alturas...",
                "Höhenkarte öffnen...",
                "Ouvrir une carte de hauteurs...",
                "Apri mappa delle altezze...",
                "高さマップを開く...",
                "打开高度图...",
                "Abrir mapa de alturas...",
            ],
            Msg::Simulate => [
                "Simulate a table",
                "Simular una mesa",
                "Tisch simulieren",
                "Simuler une table",
                "Simula un piano",
                "テーブルをシミュレート",
                "模拟工作台",
                "Simular uma mesa",
            ],
            Msg::Compute => [
                "Level the program",
                "Nivelar el programa",
                "Programm nivellieren",
                "Niveler le programme",
                "Livella il programma",
                "プログラムを補正",
                "校平程序",
                "Nivelar o programa",
            ],
            Msg::ExportProgram => [
                "Export program...",
                "Exportar programa...",
                "Programm exportieren...",
                "Exporter le programme...",
                "Esporta il programma...",
                "プログラムを書き出し...",
                "导出程序...",
                "Exportar programa...",
            ],
            Msg::ExportMap => [
                "Export map...",
                "Exportar mapa...",
                "Karte exportieren...",
                "Exporter la carte...",
                "Esporta la mappa...",
                "マップを書き出し...",
                "导出高度图...",
                "Exportar mapa...",
            ],
            Msg::TitleOpenProgram => [
                "Open a G-code program",
                "Abrir un programa G-code",
                "G-Code-Programm öffnen",
                "Ouvrir un programme G-code",
                "Apri un programma G-code",
                "G-code プログラムを開く",
                "打开 G 代码程序",
                "Abrir um programa G-code",
            ],
            Msg::TitleOpenMap => [
                "Open a height map",
                "Abrir un mapa de alturas",
                "Höhenkarte öffnen",
                "Ouvrir une carte de hauteurs",
                "Apri una mappa delle altezze",
                "高さマップを開く",
                "打开高度图",
                "Abrir um mapa de alturas",
            ],
            Msg::TitleExportProgram => [
                "Write the levelled program",
                "Escribir el programa nivelado",
                "Nivelliertes Programm schreiben",
                "Écrire le programme nivelé",
                "Scrivi il programma livellato",
                "補正したプログラムを書き込む",
                "写入校平后的程序",
                "Escrever o programa nivelado",
            ],
            Msg::TitleExportMap => [
                "Write the height map",
                "Escribir el mapa de alturas",
                "Höhenkarte schreiben",
                "Écrire la carte de hauteurs",
                "Scrivi la mappa delle altezze",
                "高さマップを書き込む",
                "写入高度图",
                "Escrever o mapa de alturas",
            ],
            Msg::SectionFiles => [
                "Files",
                "Ficheros",
                "Dateien",
                "Fichiers",
                "File",
                "ファイル",
                "文件",
                "Ficheiros",
            ],
            Msg::SectionParameters => [
                "Levelling",
                "Nivelación",
                "Nivellierung",
                "Nivellement",
                "Livellamento",
                "レベリング",
                "调平",
                "Nivelamento",
            ],
            Msg::SectionSimulation => [
                "Simulated table",
                "Mesa simulada",
                "Simulierter Tisch",
                "Table simulée",
                "Piano simulato",
                "シミュレートしたテーブル",
                "模拟工作台",
                "Mesa simulada",
            ],
            Msg::SectionMap => [
                "The map",
                "El mapa",
                "Die Karte",
                "La carte",
                "La mappa",
                "マップ",
                "高度图",
                "O mapa",
            ],
            Msg::SectionLayers => [
                "Layers",
                "Capas",
                "Ebenen",
                "Calques",
                "Livelli",
                "レイヤー",
                "图层",
                "Camadas",
            ],
            Msg::Segment => [
                "Longest piece (mm)",
                "Pieza más larga (mm)",
                "Längstes Stück (mm)",
                "Morceau le plus long (mm)",
                "Pezzo più lungo (mm)",
                "最長の分割長 (mm)",
                "最长分段 (mm)",
                "Troço mais longo (mm)",
            ],
            Msg::RefX => [
                "Reference point X (mm)",
                "Punto de referencia X (mm)",
                "Referenzpunkt X (mm)",
                "Point de référence X (mm)",
                "Punto di riferimento X (mm)",
                "基準点 X (mm)",
                "参考点 X (mm)",
                "Ponto de referência X (mm)",
            ],
            Msg::RefY => [
                "Reference point Y (mm)",
                "Punto de referencia Y (mm)",
                "Referenzpunkt Y (mm)",
                "Point de référence Y (mm)",
                "Punto di riferimento Y (mm)",
                "基準点 Y (mm)",
                "参考点 Y (mm)",
                "Ponto de referência Y (mm)",
            ],
            Msg::MaxLines => [
                "Most output lines",
                "Máximo de líneas de salida",
                "Höchstzahl Ausgabezeilen",
                "Maximum de lignes de sortie",
                "Massimo di righe di uscita",
                "出力の最大行数",
                "输出最大行数",
                "Máximo de linhas de saída",
            ],
            Msg::Method => [
                "Surface between the points",
                "Superficie entre los puntos",
                "Fläche zwischen den Punkten",
                "Surface entre les points",
                "Superficie tra i punti",
                "点の間の曲面",
                "点之间的曲面",
                "Superfície entre os pontos",
            ],
            Msg::Bilinear => [
                "Bilinear",
                "Bilineal",
                "Bilinear",
                "Bilinéaire",
                "Bilineare",
                "双線形",
                "双线性",
                "Bilinear",
            ],
            Msg::Bicubic => [
                "Bicubic (Catmull-Rom)",
                "Bicúbica (Catmull-Rom)",
                "Bikubisch (Catmull-Rom)",
                "Bicubique (Catmull-Rom)",
                "Bicubica (Catmull-Rom)",
                "双三次 (Catmull-Rom)",
                "双三次 (Catmull-Rom)",
                "Bicúbica (Catmull-Rom)",
            ],
            Msg::SimX0 => [
                "X start (mm)",
                "X inicial (mm)",
                "X Anfang (mm)",
                "X de départ (mm)",
                "X iniziale (mm)",
                "X 開始 (mm)",
                "X 起点 (mm)",
                "X inicial (mm)",
            ],
            Msg::SimY0 => [
                "Y start (mm)",
                "Y inicial (mm)",
                "Y Anfang (mm)",
                "Y de départ (mm)",
                "Y iniziale (mm)",
                "Y 開始 (mm)",
                "Y 起点 (mm)",
                "Y inicial (mm)",
            ],
            Msg::SimX1 => [
                "X end (mm)",
                "X final (mm)",
                "X Ende (mm)",
                "X de fin (mm)",
                "X finale (mm)",
                "X 終了 (mm)",
                "X 终点 (mm)",
                "X final (mm)",
            ],
            Msg::SimY1 => [
                "Y end (mm)",
                "Y final (mm)",
                "Y Ende (mm)",
                "Y de fin (mm)",
                "Y finale (mm)",
                "Y 終了 (mm)",
                "Y 终点 (mm)",
                "Y final (mm)",
            ],
            Msg::SimCols => [
                "Points along X",
                "Puntos en X",
                "Punkte entlang X",
                "Points selon X",
                "Punti lungo X",
                "X 方向の点数",
                "X 方向点数",
                "Pontos em X",
            ],
            Msg::SimRows => [
                "Points along Y",
                "Puntos en Y",
                "Punkte entlang Y",
                "Points selon Y",
                "Punti lungo Y",
                "Y 方向の点数",
                "Y 方向点数",
                "Pontos em Y",
            ],
            Msg::MeasureHint => [
                "A real map comes from the probe of the suite; here it is opened from a file or simulated",
                "Un mapa real viene de la sonda de la suite; aquí se abre de un fichero o se simula",
                "Eine echte Karte stammt vom Taster der Suite; hier wird sie aus einer Datei geöffnet oder simuliert",
                "Une vraie carte vient de la sonde de la suite ; ici elle s'ouvre depuis un fichier ou se simule",
                "Una mappa reale viene dalla sonda della suite; qui si apre da un file o si simula",
                "実際のマップはスイートのプローブから得られます。ここではファイルから開くかシミュレートします",
                "真实的高度图来自套件的探针；此处从文件打开或模拟",
                "Um mapa real vem da sonda da suite; aqui abre-se de um ficheiro ou simula-se",
            ],
            Msg::ShowMap => [
                "Height map",
                "Mapa de alturas",
                "Höhenkarte",
                "Carte de hauteurs",
                "Mappa delle altezze",
                "高さマップ",
                "高度图",
                "Mapa de alturas",
            ],
            Msg::ShowOriginal => [
                "Original program",
                "Programa original",
                "Originalprogramm",
                "Programme d'origine",
                "Programma originale",
                "元のプログラム",
                "原始程序",
                "Programa original",
            ],
            Msg::ShowCorrected => [
                "Levelled program",
                "Programa nivelado",
                "Nivelliertes Programm",
                "Programme nivelé",
                "Programma livellato",
                "補正後のプログラム",
                "校平后的程序",
                "Programa nivelado",
            ],
            Msg::LegendLow => [
                "low",
                "bajo",
                "tief",
                "bas",
                "basso",
                "低",
                "低",
                "baixo",
            ],
            Msg::LegendHigh => [
                "high",
                "alto",
                "hoch",
                "haut",
                "alto",
                "高",
                "高",
                "alto",
            ],
            Msg::LegendTable => [
                "Height of the table",
                "Altura de la mesa",
                "Höhe des Tisches",
                "Hauteur de la table",
                "Altezza del piano",
                "テーブルの高さ",
                "工作台高度",
                "Altura da mesa",
            ],
            Msg::LegendOriginal => [
                "Original path",
                "Trayectoria original",
                "Originalbahn",
                "Trajet d'origine",
                "Percorso originale",
                "元のパス",
                "原始路径",
                "Trajetória original",
            ],
            Msg::LegendCorrected => [
                "Levelled path (coloured by the table under it)",
                "Trayectoria nivelada (coloreada según la mesa)",
                "Nivellierte Bahn (nach dem Tisch darunter gefärbt)",
                "Trajet nivelé (coloré selon la table)",
                "Percorso livellato (colorato secondo il piano)",
                "補正後のパス（下のテーブルの高さで色分け）",
                "校平后的路径（按下方工作台高度着色）",
                "Trajetória nivelada (colorida conforme a mesa)",
            ],
            Msg::DropHint => [
                "Open or drop a G-code program and a height map here",
                "Abre o suelta aquí un programa G-code y un mapa de alturas",
                "Öffnen oder legen Sie hier ein G-Code-Programm und eine Höhenkarte ab",
                "Ouvrez ou déposez ici un programme G-code et une carte de hauteurs",
                "Apri o trascina qui un programma G-code e una mappa delle altezze",
                "G-code プログラムと高さマップをここに開く、またはドロップしてください",
                "在此打开或拖入 G 代码程序和高度图",
                "Abra ou largue aqui um programa G-code e um mapa de alturas",
            ],
            Msg::InfoPoints => [
                "{0} of {1} points measured",
                "{0} de {1} puntos medidos",
                "{0} von {1} Punkten gemessen",
                "{0} points mesurés sur {1}",
                "{0} punti misurati su {1}",
                "{1} 点中 {0} 点を測定",
                "已测量 {0} / {1} 个点",
                "{0} de {1} pontos medidos",
            ],
            Msg::InfoSpread => [
                "Lowest {0} mm, highest {1} mm, spread {2} mm",
                "Más bajo {0} mm, más alto {1} mm, diferencia {2} mm",
                "Tiefster {0} mm, höchster {1} mm, Spanne {2} mm",
                "Le plus bas {0} mm, le plus haut {1} mm, écart {2} mm",
                "Minimo {0} mm, massimo {1} mm, scarto {2} mm",
                "最低 {0} mm、最高 {1} mm、差 {2} mm",
                "最低 {0} mm，最高 {1} mm，差 {2} mm",
                "Mais baixo {0} mm, mais alto {1} mm, diferença {2} mm",
            ],
            Msg::InfoTilt => [
                "Tilt {0} mm/m along X, {1} mm/m along Y",
                "Inclinación {0} mm/m en X, {1} mm/m en Y",
                "Neigung {0} mm/m in X, {1} mm/m in Y",
                "Inclinaison {0} mm/m selon X, {1} mm/m selon Y",
                "Inclinazione {0} mm/m in X, {1} mm/m in Y",
                "傾き X 方向 {0} mm/m、Y 方向 {1} mm/m",
                "倾斜 X 方向 {0} mm/m，Y 方向 {1} mm/m",
                "Inclinação {0} mm/m em X, {1} mm/m em Y",
            ],
            Msg::ProgramOpened => [
                "{0}: {1} lines",
                "{0}: {1} líneas",
                "{0}: {1} Zeilen",
                "{0} : {1} lignes",
                "{0}: {1} righe",
                "{0}: {1} 行",
                "{0}：{1} 行",
                "{0}: {1} linhas",
            ],
            Msg::MapOpened => [
                "{0}: {1} of {2} points measured",
                "{0}: {1} de {2} puntos medidos",
                "{0}: {1} von {2} Punkten gemessen",
                "{0} : {1} points mesurés sur {2}",
                "{0}: {1} punti misurati su {2}",
                "{0}: {2} 点中 {1} 点を測定",
                "{0}：已测量 {1} / {2} 个点",
                "{0}: {1} de {2} pontos medidos",
            ],
            Msg::Simulated => [
                "Simulated table: {0} points",
                "Mesa simulada: {0} puntos",
                "Simulierter Tisch: {0} Punkte",
                "Table simulée : {0} points",
                "Piano simulato: {0} punti",
                "シミュレートしたテーブル: {0} 点",
                "模拟工作台：{0} 个点",
                "Mesa simulada: {0} pontos",
            ],
            Msg::Computed => [
                "{0} moves corrected, {1} pieces added, Z shifted from {2} to {3} mm",
                "{0} movimientos corregidos, {1} piezas añadidas, Z desplazada de {2} a {3} mm",
                "{0} Bewegungen korrigiert, {1} Stücke hinzugefügt, Z verschoben von {2} bis {3} mm",
                "{0} déplacements corrigés, {1} morceaux ajoutés, Z décalé de {2} à {3} mm",
                "{0} movimenti corretti, {1} pezzi aggiunti, Z spostata da {2} a {3} mm",
                "{0} 件の移動を補正、{1} 個の分割を追加、Z のずれ {2} ～ {3} mm",
                "已校正 {0} 次移动，新增 {1} 段，Z 偏移 {2} 至 {3} mm",
                "{0} movimentos corrigidos, {1} troços acrescentados, Z deslocado de {2} a {3} mm",
            ],
            Msg::NeedProgram => [
                "Open a G-code program first",
                "Abre primero un programa G-code",
                "Öffnen Sie zuerst ein G-Code-Programm",
                "Ouvrez d'abord un programme G-code",
                "Apri prima un programma G-code",
                "先に G-code プログラムを開いてください",
                "请先打开 G 代码程序",
                "Abra primeiro um programa G-code",
            ],
            Msg::NeedMap => [
                "Open or simulate a height map first",
                "Abre o simula primero un mapa de alturas",
                "Öffnen oder simulieren Sie zuerst eine Höhenkarte",
                "Ouvrez ou simulez d'abord une carte de hauteurs",
                "Apri o simula prima una mappa delle altezze",
                "先に高さマップを開くかシミュレートしてください",
                "请先打开或模拟高度图",
                "Abra ou simule primeiro um mapa de alturas",
            ],
            Msg::Truncated => [
                "The picture shows only the first {0} segments",
                "El dibujo solo muestra los primeros {0} segmentos",
                "Die Zeichnung zeigt nur die ersten {0} Segmente",
                "Le dessin ne montre que les {0} premiers segments",
                "Il disegno mostra solo i primi {0} segmenti",
                "図には最初の {0} 区間のみ表示します",
                "图中仅显示前 {0} 段",
                "O desenho mostra apenas os primeiros {0} segmentos",
            ],
            Msg::CursorTable => [
                "X {0}  Y {1}  table {2} mm",
                "X {0}  Y {1}  mesa {2} mm",
                "X {0}  Y {1}  Tisch {2} mm",
                "X {0}  Y {1}  table {2} mm",
                "X {0}  Y {1}  piano {2} mm",
                "X {0}  Y {1}  テーブル {2} mm",
                "X {0}  Y {1}  工作台 {2} mm",
                "X {0}  Y {1}  mesa {2} mm",
            ],
            Msg::SimulatedName => [
                "(simulated table)",
                "(mesa simulada)",
                "(simulierter Tisch)",
                "(table simulée)",
                "(piano simulato)",
                "（シミュレートしたテーブル）",
                "（模拟工作台）",
                "(mesa simulada)",
            ],
            Msg::BadFile => [
                "{0}: not a valid file of this kind",
                "{0}: no es un fichero válido de este tipo",
                "{0}: keine gültige Datei dieser Art",
                "{0} : ce n'est pas un fichier valide de ce type",
                "{0}: non è un file valido di questo tipo",
                "{0}: この種類の有効なファイルではありません",
                "{0}：不是此类有效文件",
                "{0}: não é um ficheiro válido deste tipo",
            ],
        }
    }

    /// The text in a language.
    pub fn tr(self, lang: Lang) -> &'static str {
        self.words()[lang as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_sentence_has_a_text_in_each_of_the_eight_languages() {
        for m in Msg::ALL {
            for l in Lang::ALL {
                assert!(!m.tr(l).trim().is_empty(), "{m:?} in {}", l.code());
            }
        }
    }

    #[test]
    fn the_list_of_sentences_has_no_repeats() {
        for (i, a) in Msg::ALL.iter().enumerate() {
            assert!(!Msg::ALL[i + 1..].contains(a), "{a:?} is listed twice");
        }
    }

    #[test]
    fn a_template_has_the_same_places_in_every_language() {
        for m in Msg::ALL {
            let places = |l: Lang| {
                (0..4)
                    .filter(|k| m.tr(l).contains(&format!("{{{k}}}")))
                    .count()
            };
            for l in Lang::ALL {
                assert_eq!(places(l), places(Lang::En), "{m:?} in {}", l.code());
            }
        }
    }

    #[test]
    fn the_order_of_the_languages_matches_the_lists() {
        for (k, l) in Lang::ALL.into_iter().enumerate() {
            assert_eq!(l as usize, k);
        }
        assert_eq!(Msg::Cancel.tr(Lang::Es), "Cancelar");
        assert_eq!(Msg::Cancel.tr(Lang::De), "Abbrechen");
        assert_eq!(Msg::Cancel.tr(Lang::Pt), "Cancelar");
    }

    #[test]
    fn a_language_is_found_from_the_code_of_the_system() {
        assert_eq!(Lang::from_code("es_ES.UTF-8"), Some(Lang::Es));
        assert_eq!(Lang::from_code("PT-pt"), Some(Lang::Pt));
        assert_eq!(Lang::from_code("ja"), Some(Lang::Ja));
        assert_eq!(Lang::from_code("xx"), None);
        assert_eq!(Lang::from_code(""), None);
    }

    #[test]
    fn the_places_of_a_template_are_filled() {
        assert_eq!(fill("{1} and {0}", &["a", "b"]), "b and a");
        assert_eq!(fill("none", &["a"]), "none");
    }
}
