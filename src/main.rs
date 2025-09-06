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
    units: Vec<[usize; 9]>, // 27 ユニット (行9, 列9, ブロック9)
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
fn precompute() -> (Vec<Vec<usize>>, Vec<[usize; 9]>) {
    // 27 ユニット x 9 セルを配列で保持
    let mut units: Vec<[usize; 9]> = Vec::with_capacity(27);

    // 行
    for r in 0..9 {
        let mut u = [0usize; 9];
        for c in 0..9 {
            u[c] = r * 9 + c;
        }
        units.push(u);
    }
    // 列
    for c in 0..9 {
        let mut u = [0usize; 9];
        for r in 0..9 {
            u[r] = r * 9 + c;
        }
        units.push(u);
    }
    // ブロック
    for br in 0..3 {
        for bc in 0..3 {
            let mut u = [0usize; 9];
            let mut k = 0;
            for dr in 0..3 {
                for dc in 0..3 {
                    let r = br * 3 + dr;
                    let c = bc * 3 + dc;
                    u[k] = r * 9 + c;
                    k += 1;
                }
            }
            units.push(u);
        }
    }
    // peers: 各セルの仲間セルを収集（bool フラグで重複除去）
    let mut peers: Vec<Vec<usize>> = Vec::with_capacity(81);
    for i in 0..81 {
        // 81個のboolフラグ配列。trueになったセルがiのpeerになる
        let mut mark = [false; 81];

        // このセルが属する 3 つのユニット（行/列/ブロック）を特定
        let r = i / 9; // セルiの行番号（0-8）
        let c = i % 9; // セルiの列番号（0-8）
        let row_ui = r; // 行ユニットのインデックス（0..=8）
        let col_ui = 9 + c; // 列ユニットのインデックス（9..=17）
        let block_ui = 18 + (r / 3) * 3 + (c / 3); // ブロックユニットのインデックス（18..=26）

        // デバッグ出力（最初の数個のセルのみ）
        if i < 5 {
            println!("セル {} (行{}, 列{}) の計算:", i, r, c);
            println!("  行ユニット[{}]: {:?}", row_ui, units[row_ui]);
            println!("  列ユニット[{}]: {:?}", col_ui, units[col_ui]);
            println!("  ブロックユニット[{}]: {:?}", block_ui, units[block_ui]);
        }

        // 3つのユニット（行、列、ブロック）のそれぞれについて
        for &ui in [row_ui, col_ui, block_ui].iter() {
            // そのユニット内の全セルをmarkに記録（自分自身は除く）
            for &cell in units[ui].iter() {
                if cell != i {
                    mark[cell] = true;
                    if i < 5 {
                        println!("    ユニット[{}] から セル {} をpeerに追加", ui, cell);
                    }
                }
            }
        }

        // 安定した順序（インデックス順）でmarkされたセルを収集
        let mut v = Vec::with_capacity(20); // 通常20個のpeer
        for idx in 0..81 {
            if mark[idx] {
                v.push(idx);
            }
        }

        if i < 5 {
            println!("  最終的なpeers({} 個): {:?}\n", v.len(), v);
        }

        peers.push(v);
    }

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

    let _sdk = Sudoku::from_string(p);
}
