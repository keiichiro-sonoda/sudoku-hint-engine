type Mask = u16;
const ALL: Mask = 0x3FE; // bits 1..=9

#[derive(Clone)]
pub struct Sudoku {
    /// 各マスの候補を表すビットマスク。インデックスは `r*9 + c` (0-based)。
    /// 例: 先頭左上は 0、右隣は 1、…、次の行は +9。
    cells: [Mask; 81],

    /// peers[i]: マス i と同じ「行・列・ブロック」に属する全てのマスの集合。
    /// - 自身 i は含まない。
    /// - 典型的には 20 個（行8 + 列8 + ブロック8 − 重複）程度。
    /// - 値の更新/確定時に、この集合の候補から同じ数字を除外するのに使う。
    peers: Vec<Vec<usize>>,

    /// units[u]: 盤面の「ユニット（制約集合）」を列挙した配列。
    /// - 全部で 27 ユニット（行 9、列 9、3x3 ブロック 9）。
    /// - 各ユニットは 9 マス分のインデックスを持つ（`usize` の 9 要素）。
    /// - 例えば「行の中でその数字が入る場所は1つだけ」のような一意性チェックに使う。
    units: Vec<Vec<usize>>, // 27 ユニット (行9, 列9, ブロック9)
}

impl Sudoku {
    pub fn new() -> Self {
        // precompute() では、固定の盤面構造から以下を計算して返す:
        // - peers: 長さ 81。各 i について、同じ行・列・ブロックにあるマスのインデックス列。
        // - units: 長さ 27。行9/列9/ブロック9 の各ユニットを 9 マスのインデックスで表現。
        let (peers, units) = precompute();
        Self {
            cells: [ALL; 81],
            peers,
            units,
        }
    }

    pub fn from_string(s: &str) -> Self {
        // 81文字 (. or 0 は空、1..9 は確定)
        let sdk = Sudoku::new();
        let bytes: Vec<u8> = s.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
        assert!(bytes.len() == 81, "Need 81 chars (digits or '.'/'0')");
        sdk
    }
}

/// 数独盤面の構造情報を事前計算する。
///
/// # 戻り値
///
/// - `peers`: 各セル（81個）について、同じ行・列・ブロックに属する他のセルのインデックス集合
/// - `units`: 全ユニット（27個）のセルインデックス集合
///   - 行 9個（各行の9セル）
///   - 列 9個（各列の9セル）
///   - 3x3ブロック 9個（各ブロックの9セル）
///
/// # 実装詳細
///
/// セルのインデックスは `r*9 + c` （r=行, c=列, 0-based）で計算される。
/// peersは各セルについて、制約違反チェック時に参照する必要がある
/// 他のセル群を効率的に取得するために使用される。
fn precompute() -> (Vec<Vec<usize>>, Vec<Vec<usize>>) {
    // 行のユニット（0-8行）
    let row_units: Vec<Vec<usize>> = (0..9)
        .map(|r| (0..9).map(|c| r * 9 + c).collect())
        .collect();

    // 列のユニット（0-8列）
    let col_units: Vec<Vec<usize>> = (0..9)
        .map(|c| (0..9).map(|r| r * 9 + c).collect())
        .collect();

    // 3x3ブロックのユニット（9個）
    let block_units: Vec<Vec<usize>> = (0..3)
        .flat_map(|block_r| {
            (0..3).map(move |block_c| {
                (0..3)
                    .flat_map(|r| (0..3).map(move |c| (block_r * 3 + r) * 9 + (block_c * 3 + c)))
                    .collect()
            })
        })
        .collect();

    let units = [row_units, col_units, block_units].concat();

    // peersの計算：各セルについて同じユニットに属する他のセルを収集
    let peers: Vec<Vec<usize>> = (0..81)
        .map(|i| {
            units
                .iter()
                .filter(|unit| unit.contains(&i))
                .flat_map(|unit| unit.iter().copied())
                .filter(|&cell| cell != i)
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect()
        })
        .collect();

    (peers, units)
}

fn main() {
    let p = "\
    .4.....8.\
    6..2....3\
    .893.....\
    .....8.6.\
    ..1......\
    ...754.9.\
    .....6...\
    .2....64.\
    ..31....5\
    ";
    println!("{}", p);

    // precomputeの結果をデバッグ出力
    let (peers, units) = precompute();

    println!("\n=== Units ===");
    println!("行のユニット (0-8):");
    for (i, unit) in units.iter().take(9).enumerate() {
        println!("  行{}: {:?}", i, unit);
    }

    println!("列のユニット (0-8):");
    for (i, unit) in units.iter().skip(9).take(9).enumerate() {
        println!("  列{}: {:?}", i, unit);
    }

    println!("3x3ブロックのユニット (0-8):");
    for (i, unit) in units.iter().skip(18).enumerate() {
        println!("  ブロック{}: {:?}", i, unit);
    }

    println!("\n=== Peers (例: いくつかのセル) ===");
    // いくつかの代表的なセルのpeersを表示
    let test_cells = [0, 4, 40, 80]; // 左上角、上中央、中央、右下角
    for &cell in &test_cells {
        let (r, c) = (cell / 9, cell % 9);
        println!(
            "セル({},{}) [index={}] のpeers({})個: {:?}",
            r,
            c,
            cell,
            peers[cell].len(),
            peers[cell]
        );
    }

    let _sdk = Sudoku::from_string(p);
}
