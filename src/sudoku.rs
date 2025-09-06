use crate::strategy::Mask;

pub const ALL: Mask = 0x3FE; // bits 1..=9

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

    /// 初期盤面で確定済みのセルを記録（Naked Singleから除外するため）
    initially_given: [bool; 81], // true = 初期から確定, false = 推論で確定

    /// 現在の盤面で確定済みのセルを記録（初期・推論を問わず確定）
    confirmed: [bool; 81],
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
            initially_given: [false; 81],
            confirmed: [false; 81],
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
                // 初期盤面で確定済みとしてマーク
                sdk.initially_given[i] = true;
                sdk.confirmed[i] = true;
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

    /// 新しいStrategy基盤用: セルに値を割り当て
    ///
    /// - `set_value` で確定し、関連セルから候補を除去します。
    /// - 同じセルに対して繰り返し割り当てないよう、`confirmed` を `true` にします。
    pub fn assign(&mut self, cell: usize, digit: u8) -> Result<(), ()> {
        self.set_value(cell, digit as usize);
        // 以後 同じセルを繰り返し提案しないようにする
        self.confirmed[cell] = true;
        Ok(())
    }

    /// 新しいStrategy基盤用: セルから候補を除外
    pub fn eliminate(&mut self, cell: usize, digit: u8) -> Result<(), ()> {
        let mask = 1u16 << digit;
        self.cells[cell] &= !mask;
        if self.cells[cell] == 0 {
            return Err(()); // 候補が全部なくなった = 矛盾
        }
        Ok(())
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

    /// セルのマスクを取得（外部からアクセス用）
    pub fn cell_mask(&self, cell: usize) -> Mask {
        self.cells[cell]
    }

    /// unitsを取得（外部からアクセス用）
    pub fn units(&self) -> &Vec<[usize; 9]> {
        &self.units
    }

    /// initially_givenを確認（外部からアクセス用）
    pub fn initially_given(&self, cell: usize) -> bool {
        self.initially_given[cell]
    }

    /// confirmed（確定済みか）を確認（外部からアクセス用）
    pub fn is_confirmed(&self, cell: usize) -> bool {
        self.confirmed[cell]
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
        let mut mark = [false; 81];

        // このセルが属する 3 つのユニット（行/列/ブロック）を特定
        let r = i / 9;
        let c = i % 9;
        let row_ui = r; // 0..=8
        let col_ui = 9 + c; // 9..=17
        let block_ui = 18 + (r / 3) * 3 + (c / 3); // 18..=26

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
