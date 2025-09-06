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

    /// 初期盤面で確定済みのセルを記録（Naked Singleから除外するため）
    initially_given: [bool; 81], // true = 初期から確定, false = 推論で確定
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
    pub fn assign(&mut self, cell: usize, digit: u8) -> Result<(), ()> {
        self.set_value(cell, digit as usize);
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

// ===== Strategy 基盤 =====

/// 数独解法手筋の共通インターフェース
pub trait Strategy {
    fn name(&self) -> &'static str;

    /// 最初に見つかったヒント1つを返す（パフォーマンス重視）
    fn find(&self, sdk: &Sudoku) -> Option<Hint>;

    /// その手筋で見つかる全てのヒントを返す（学習・分析用）
    fn find_all(&self, sdk: &Sudoku) -> Vec<Hint>;
}

/// 手筋の結果を表すヒント構造体
#[derive(Clone, Debug)]
pub struct Hint {
    pub description: String,
    pub assignments: Vec<(usize, u8)>,       // (セル, 数字) の確定
    pub eliminations: Vec<(usize, Vec<u8>)>, // (セル, [除外数字リスト])
}

impl Hint {
    /// ヒントを実際の盤面に適用
    pub fn apply(self, sdk: &mut Sudoku) -> Result<(), ()> {
        for (i, d) in self.assignments {
            sdk.assign(i, d)?;
        }
        for (i, ds) in self.eliminations {
            for d in ds {
                sdk.eliminate(i, d)?;
            }
        }
        Ok(())
    }
}

// ===== ヘルパー関数 =====

/// ビットマスクが1つのbitだけ立っているかチェック
fn is_single(mask: Mask) -> bool {
    mask.count_ones() == 1
}

/// 1つのbitだけ立っているマスクから数字を取得
fn single_digit(mask: Mask) -> Option<u8> {
    if !is_single(mask) {
        return None;
    }
    for d in 1..=9 {
        if mask & bit(d) != 0 {
            return Some(d);
        }
    }
    None
}

/// 数字からビットマスクを生成
fn bit(digit: u8) -> Mask {
    1u16 << digit
}

// ===== 具体的な手筋 =====

/// Naked Single: 候補が1つのセルは確定
///
/// ヒント内容: 「あるマスに注目して、このマスにはこれしか入らない」
///
/// 人が頭の中でやっていること（直感的な説明）
/// - 1つのマスを見て、そのマスに入れられる数字の候補（1〜9）を考える。
/// - 候補が1個しか残っていなければ、そのマスはその数字で“確定”。
///
/// プログラムでの実装（機械化）
/// - 盤面はu16のビットマスクで候補を保持（bit1〜bit9 = 数字1〜9）。
/// - `count_ones()` で候補数を数え、bitが立っている数字を列挙。
/// - 候補がちょうど1つならヒントを返す（確定自体はしない）。
pub struct NakedSingle;

impl Strategy for NakedSingle {
    fn name(&self) -> &'static str {
        "Naked Single"
    }

    fn find(&self, sdk: &Sudoku) -> Option<Hint> {
        // find_allの最初の1つを返す
        self.find_all(sdk).into_iter().next()
    }

    fn find_all(&self, sdk: &Sudoku) -> Vec<Hint> {
        let mut hints = Vec::new();

        for i in 0..81 {
            // 初期盤面で既に確定済みのセルは除外
            if !sdk.initially_given[i] && is_single(sdk.cells[i]) {
                let d = single_digit(sdk.cells[i]).unwrap();
                hints.push(Hint {
                    description: format!(
                        "セル({},{}) は候補が1つ {} なので確定（Naked Single）",
                        i / 9 + 1,
                        i % 9 + 1,
                        d
                    ),
                    assignments: vec![(i, d)],
                    eliminations: vec![],
                });
            }
        }

        hints
    }
}

/// Hidden Single: ユニット内で特定の数字を置ける場所が1つ
///
/// ヒント内容: 「ある数字に注目して、この列/行/ブロックではここにしか置けない」
///
/// 人の思考（例: 数字7）
/// 1. 着目: 「7について考える」
/// 2. 制約確認: 既にある数字により、行/列/ブロック内で入れない場所を消す
/// 3. 候補絞り込み: 行/列/ブロックの中で7を置ける位置を探す
/// 4. 発見: 入る位置が1つだけなら「ここにしか置けない」
///
/// プログラムでの実装（機械化）
/// - 各ユニット（行/列/ブロック）ごとに、数字1..=9を走査。
/// - `cells[cell] & (1<<digit) != 0` で、その数字が候補に残るセルのみ抽出。
/// - 抽出結果が1件なら、その位置と数字でヒントを返す（確定自体はしない）。
pub struct HiddenSingle;

impl Strategy for HiddenSingle {
    fn name(&self) -> &'static str {
        "Hidden Single"
    }

    fn find(&self, sdk: &Sudoku) -> Option<Hint> {
        // find_allの最初の1つを返す
        self.find_all(sdk).into_iter().next()
    }

    fn find_all(&self, sdk: &Sudoku) -> Vec<Hint> {
        let mut hints = Vec::new();

        // 【人間思考ステップ1】各制約ユニット（行/列/ブロック）を順番に調べる
        // 「まず行1を見てみよう」「次に列1を見てみよう」「ブロック(1,1)も確認」
        for (unit_index, unit) in sdk.units.iter().enumerate() {
            // 【人間思考ステップ2】そのユニット内で数字1〜9を順番に考える
            // 「この行で数字1はどこに入るかな？」「数字2はどこ？」
            for digit in 1..=9 {
                // 【人間思考ステップ3】その数字が入れられる候補場所を全て探す
                // 「1が入るのは...ここと、ここと、あとここかな」
                let possible_cells = self.find_cells_that_can_hold_digit(unit, digit, sdk);

                // 【人間思考ステップ4】候補場所が1つだけなら発見！
                // 「あ、1が入れるのはここだけだ！確定だね」
                if possible_cells.len() == 1 {
                    let target_cell = possible_cells[0];

                    // 既にその場所が確定済みでなければヒント生成
                    // （確定済みなら当然そこにしか入らないので、ヒントとしては意味ない）
                    if !is_single(sdk.cells[target_cell]) {
                        let unit_description = self.describe_unit(unit_index);
                        let (row, col) = (target_cell / 9 + 1, target_cell % 9 + 1);

                        hints.push(Hint {
                            description: format!(
                                "{} で数字 {} を置けるのはセル({},{}) だけ（Hidden Single）",
                                unit_description, digit, row, col
                            ),
                            assignments: vec![(target_cell, digit)],
                            eliminations: vec![],
                        });
                    }
                }
            }
        }

        hints
    }
}

impl HiddenSingle {
    /// 【ヘルパー関数】特定のユニット内で、特定の数字が入れられるセルを全て探す
    /// 人間が「この行で7が入るのはどこどこ？」と候補を数え上げる処理
    fn find_cells_that_can_hold_digit(
        &self,
        unit: &[usize; 9],
        digit: u8,
        sdk: &Sudoku,
    ) -> Vec<usize> {
        let digit_mask = bit(digit);
        let mut possible_cells = Vec::new();

        // ユニット内の9マスを順番にチェック
        for &cell_index in unit {
            // そのセルの候補マスクを確認
            // 「このマスに7は入るかな？候補に7が残ってるかな？」
            if sdk.cells[cell_index] & digit_mask != 0 {
                possible_cells.push(cell_index);
            }
        }

        possible_cells
    }

    /// 【ヘルパー関数】ユニットインデックスから人間にわかりやすい名前を生成
    /// 0 → "行 1"、9 → "列 1"、18 → "ブロック(1, 1)" など
    fn describe_unit(&self, unit_index: usize) -> String {
        match unit_index {
            // 行のユニット (インデックス 0-8)
            0..=8 => format!("行 {}", unit_index + 1),

            // 列のユニット (インデックス 9-17)
            9..=17 => format!("列 {}", unit_index - 9 + 1),

            // 3x3ブロックのユニット (インデックス 18-26)
            _ => {
                let block_index = unit_index - 18;
                let block_row = block_index / 3 + 1; // 1, 2, 3
                let block_col = block_index % 3 + 1; // 1, 2, 3
                format!("ブロック({}, {})", block_row, block_col)
            }
        }
    }
}

// ===== ヒントを順に探して適用するエンジン =====

/// 数独解法エンジン
pub struct Engine {
    strategies: Vec<Box<dyn Strategy>>,
}

impl Engine {
    /// 基本的な手筋を使うエンジンを作成
    /// 優先順位: Hidden Single -> Naked Single
    /// (Hidden Singleの方が一般的に見つけにくいため優先)
    pub fn basic() -> Self {
        Self {
            strategies: vec![Box::new(HiddenSingle), Box::new(NakedSingle)],
        }
    }

    /// 次に使える手筋を探してヒントを返す（最初の1つ）
    pub fn next_hint(&self, sdk: &Sudoku) -> Option<Hint> {
        for s in &self.strategies {
            if let Some(h) = s.find(sdk) {
                return Some(h);
            }
        }
        None
    }

    /// 全ての手筋で見つかる全てのヒントを返す（学習・分析用）
    pub fn all_hints(&self, sdk: &Sudoku) -> Vec<(String, Vec<Hint>)> {
        self.strategies
            .iter()
            .map(|s| (s.name().to_string(), s.find_all(sdk)))
            .filter(|(_, hints)| !hints.is_empty())
            .collect()
    }
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

    println!("\n=== Strategy基盤でのヒント検出 ===");
    let engine = Engine::basic();

    // まず各手筋を個別にテスト
    let naked_single = NakedSingle;
    let hidden_single = HiddenSingle;

    println!("--- Naked Single テスト ---");
    if let Some(hint) = naked_single.find(&sdk) {
        println!("Naked Single 発見: {}", hint.description);
    } else {
        println!("Naked Single: なし");
    }

    println!("--- Hidden Single テスト ---");
    if let Some(hint) = hidden_single.find(&sdk) {
        println!("Hidden Single 発見: {}", hint.description);
    } else {
        println!("Hidden Single: なし");
    }

    // 新しい方式でヒントを探す（1つだけ）
    if let Some(hint) = engine.next_hint(&sdk) {
        println!("\nエンジンが選択: {}", hint.description);

        // ヒントを適用してみる
        println!("ヒントを適用中...");
        let mut sdk_copy = sdk.clone();
        match hint.apply(&mut sdk_copy) {
            Ok(()) => println!("✓ ヒント適用成功"),
            Err(()) => println!("✗ ヒント適用失敗（矛盾発生）"),
        }
    } else {
        println!("新しいStrategy基盤では利用可能なヒントがありません");
    }

    println!("\n=== 全手筋での包括的ヒント検出 ===");
    let all_hints = engine.all_hints(&sdk);

    for (strategy_name, hints) in &all_hints {
        println!("--- {} ({} 個発見) ---", strategy_name, hints.len());
        for (i, hint) in hints.iter().enumerate() {
            println!("  {}. {}", i + 1, hint.description);
        }
    }

    if all_hints.is_empty() {
        println!("全ての手筋で利用可能なヒントがありません");
    }
}
