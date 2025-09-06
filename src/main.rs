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

    /// 文字列から数独の初期盤面を作ります。
    ///
    /// 初心者向けのざっくり説明:
    /// - 入力は「81文字」の盤面です（9行×9列）。
    /// - 数字 '1'〜'9' は「すでに決まっている数」。
    /// - '.'（ドット）、'0'、スペースや改行は「空きマス（未確定）」として扱います。
    /// - 決まっている数は、その行・列・ブロックの他のマスから同じ数字の候補を消します。
    ///
    /// 例（改行やスペースは自動で無視されます）:
    ///
    /// .4.....8.
    /// 6..2....3
    /// .893.....
    /// .....8.6.
    /// ..1......
    /// ...754.9.
    /// .....6...
    /// .2....64.
    /// ..31....5
    pub fn from_string(s: &str) -> Self {
        // 1) 空の盤面（全マス「1〜9 すべて候補」）を用意
        let mut sdk = Sudoku::new();

        // 2) 余計な空白・改行を取り除き、ちょうど81文字か確認
        let bytes: Vec<u8> = s.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
        assert!(bytes.len() == 81, "Need 81 chars (digits or '.'/'0')");

        // 3) 左上(インデックス0)から右下(インデックス80)まで走査し、
        //    '1'〜'9'なら確定として反映する（周囲の候補も同時に整理）
        for (i, &byte) in bytes.iter().enumerate() {
            if byte >= b'1' && byte <= b'9' {
                let digit = (byte - b'0') as usize;
                // set_value: マス i を digit で確定し、その行・列・ブロックの
                //            他マスから digit の候補を消します
                sdk.set_value(i, digit);
            } else {
                // '.' や '0' は「何も確定しない」= 全候補のまま
                // 何もしないことで未確定を表現します
            }
        }

        // 4) 初期化済みの盤面を返す
        sdk
    }

    /// セルに値を確定し、peersから候補を除外
    pub fn set_value(&mut self, cell: usize, value: usize) {
        assert!(value >= 1 && value <= 9);
        let mask = 1u16 << value;

        // セル自身を確定値のみに設定
        self.cells[cell] = mask;

        // peersから候補を除外
        for &peer in &self.peers[cell] {
            self.cells[peer] &= !mask;
        }
    }

    /// 指定されたセルの候補を表示用文字列に変換します。
    ///
    /// 初心者向けの詳しい説明:
    ///
    /// このメソッドは各マスの状態を人間にわかりやすく表示するためのものです。
    ///
    /// # ビットマスクの仕組み
    ///
    /// 各セルは `cells[cell]` という16bit整数で候補を管理しています。
    /// - bit 1が立っている → 数字1が候補
    /// - bit 2が立っている → 数字2が候補
    /// - ...
    /// - bit 9が立っている → 数字9が候補
    ///
    /// 例: mask = 0b1000001010 なら
    /// - bit 1 = 0 → 1は候補でない
    /// - bit 2 = 1 → 2は候補
    /// - bit 3 = 0 → 3は候補でない
    /// - bit 4 = 1 → 4は候補
    /// - bit 9 = 1 → 9は候補
    ///
    /// # 戻り値
    ///
    /// - 確定済み（1つの候補のみ）: "7" のような数字そのもの
    /// - 未確定（複数候補）: "[2,4,9]" のような候補リスト
    pub fn candidates_string(&self, cell: usize) -> String {
        // このセルのビットマスクを取得
        let mask = self.cells[cell];

        // count_ones() = 立っているbitの数 = 候補の数
        if mask.count_ones() == 1 {
            // ちょうど1個の候補 = 確定値
            // どの数字が確定しているか探す
            for digit in 1..=9 {
                // 1 << digit でdigit番目のbitをチェック
                // 例: digit=7なら 1<<7 = 0b10000000 (7bit目)
                if mask & (1 << digit) != 0 {
                    // このdigitのbitが立っている = 確定値はdigit
                    return digit.to_string();
                }
            }
        }

        // 複数候補がある場合: [1,3,7] のような形式で表示
        let candidates: Vec<String> = (1..=9)
            .filter(|&digit| {
                // このdigitが候補に含まれているかチェック
                mask & (1 << digit) != 0
            })
            .map(|d| d.to_string()) // 数字を文字列に変換
            .collect();

        // "[1,3,7]" のような文字列を作成
        format!("[{}]", candidates.join(","))
    }

    /// ヒント①：「ある数字に注目して、この列/行/ブロックではここにしか置けない」
    ///
    /// これは Hidden Single（ヒドゥンシングル）と呼ばれる手法です。
    /// 
    /// ## 人間の思考プロセス（例：数字7を考える場合）
    /// 
    /// 1. **着目**: 「数字7について考えてみよう」
    /// 2. **制約確認**: 盤面を見て「あ、ここに7がある。じゃあこの行・列・ブロックには他に7は入らない」
    /// 3. **候補絞り込み**: 「他の行を見てみよう。この行では7はどこに入るかな？」
    /// 4. **消去法**: 「ここは既に他の数字が入ってるからダメ」「ここも制約で7が入らない」
    /// 5. **発見**: 「あ、この行では7はここにしか入らない！確定だ」
    /// 
    /// ## プログラムでの実装（人間の思考をそのまま再現）
    /// 
    /// 1. **着目** → `for digit in 1..=9`: 数字1〜9を順番に考える
    /// 2. **制約確認** → 初期化時の`set_value`で既に処理済み（候補マスクから除外済み）
    /// 3. **候補絞り込み** → `for unit_idx in 0..self.units.len()`: 各行・列・ブロックを順番に調べる
    /// 4. **消去法** → `filter(|&&cell| self.cells[cell] & mask != 0)`: その数字が候補に残っているセルだけ抽出
    /// 5. **発見** → `if possible_cells.len() == 1`: 候補が1つだけならヒント生成
    /// 
    /// つまり、人間が「この数字はここにしか入らない」と気づく瞬間を、
    /// プログラムでは「候補リストの長さが1」として機械的に検出しています。
    pub fn find_hidden_singles(&self) -> Vec<String> {
        let mut hints = Vec::new();

        // 【人間思考3】候補絞り込み: 各行・列・ブロックを順番に調べる
        for unit_idx in 0..self.units.len() {
            let unit = &self.units[unit_idx];
            let unit_name = if unit_idx < 9 {
                format!("行{}", unit_idx)
            } else if unit_idx < 18 {
                format!("列{}", unit_idx - 9)
            } else {
                format!("ブロック{}", unit_idx - 18)
            };

            // 【人間思考1】着目: 「数字1について考えてみよう」「数字2について考えてみよう」...
            for digit in 1..=9 {
                let mask = 1u16 << digit; // この数字のビットマスクを作成
                
                // 【人間思考4】消去法: 「この行でdigitが入れるのはどこかな？」
                // → 「ここは既に他の数字が入ってるからダメ」「ここも制約でdigitが入らない」
                let possible_cells: Vec<usize> = unit
                    .iter()
                    .filter(|&&cell| self.cells[cell] & mask != 0) // digitが候補に残っているセルのみ
                    .copied()
                    .collect();

                // 【人間思考5】発見: 「あ、この行ではdigitはここにしか入らない！」
                if possible_cells.len() == 1 {
                    let cell = possible_cells[0];
                    let (r, c) = (cell / 9, cell % 9);
                    // 既に確定済みでなければヒント生成
                    if self.cells[cell].count_ones() > 1 {
                        hints.push(format!(
                            "数字{}は{}で({},{})にしか置けません",
                            digit, unit_name, r, c
                        ));
                    }
                }
            }
        }

        hints
    }

    /// ヒント②：「あるマスに注目して、このマスにはこれしか入らない」
    ///
    /// これは Naked Single（ネイキッドシングル）と呼ばれる手法です。
    ///
    /// 人が頭の中でやっていること（直感的な説明）
    /// - 1つのマスを見て、そのマスに入れられる数字の候補（1〜9）を考えます。
    /// - 候補が1個しか残っていなければ、そのマスはその数字で"確定"です。
    ///
    /// この関数での実装（プログラム的な説明）
    /// - 盤面は「ビットマスク」で候補を表現しています（u16 の bit1〜bit9 を使用）。
    ///   たとえば、bit3 が1なら「3が候補に含まれる」を意味します。
    /// - 各マスについて、マスク中の1ビット数（候補の個数）を `count_ones()` で数えます。
    ///   - すでに1個ならそのマスは確定済みなのでスキップ。
    ///   - 1個より多い場合は、bitが立っている数字だけを列挙して候補リストを作ります。
    /// - その候補リストの長さがちょうど1なら「このマスにはこの数字しか入らない」というヒント文字列を返します。
    ///
    /// 注意
    /// - この関数は「確定させる処理」はしません。あくまで"ヒント文を作るだけ"です。
    ///   実際に確定させたい場合は、別途 `set_value` を呼ぶ設計にしています。
    pub fn find_naked_singles(&self) -> Vec<String> {
        let mut hints = Vec::new();

        // 全81マスを順にチェック
        for cell in 0..81 {
            let mask = self.cells[cell];
            if mask.count_ones() == 1 {
                continue; // 既に確定しているマスはスキップ
            }

            // そのマスの「立っているビット=候補数字」を収集
            let candidates: Vec<usize> =
                (1..=9).filter(|&digit| mask & (1 << digit) != 0).collect();

            if candidates.len() == 1 {
                let digit = candidates[0];
                let (r, c) = (cell / 9, cell % 9);
                hints.push(format!("セル({},{})には{}しか入りません", r, c, digit));
            }
        }

        hints
    }

    /// 盤面を表示
    pub fn display(&self) {
        for r in 0..9 {
            for c in 0..9 {
                let cell = r * 9 + c;
                print!("{:>8}", self.candidates_string(cell));
            }
            println!();
        }
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

        // 3つのユニット（行、列、ブロック）のそれぞれについて
        for &ui in [row_ui, col_ui, block_ui].iter() {
            // そのユニット内の全セルをmarkに記録（自分自身は除く）
            for &cell in units[ui].iter() {
                if cell != i {
                    mark[cell] = true;
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
    println!("初期盤面:");
    println!("{}", p);

    let mut sdk = Sudoku::from_string(p);

    println!("\n=== 候補状態 ===");
    sdk.display();

    println!("\n=== ヒント①: 数字に注目（Hidden Singles） ===");
    let hints1 = sdk.find_hidden_singles();
    for hint in &hints1 {
        println!("{}", hint);
    }

    println!("\n=== ヒント②: マスに注目（Naked Singles） ===");
    let hints2 = sdk.find_naked_singles();
    for hint in &hints2 {
        println!("{}", hint);
    }

    if hints1.is_empty() && hints2.is_empty() {
        println!("現在利用可能な基本ヒントはありません");
    }
}
