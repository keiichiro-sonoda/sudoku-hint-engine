use crate::sudoku::Sudoku;

pub type Mask = u16;

/// 数独解法手筋の共通インターフェース
pub trait Strategy {
    fn name(&self) -> &'static str;

    /// 最初に見つかったヒント1つを返す（パフォーマンス重視）
    fn find(&self, sdk: &Sudoku) -> Option<Hint>;

    /// その手筋で見つかる全てのヒントを返す（学習・分析用）
    fn find_all(&self, sdk: &Sudoku) -> Vec<Hint>;

    /// 指定されたレベルでヒントを返す
    fn find_with_level(&self, sdk: &Sudoku, level: HintLevel) -> Option<Hint>;

    /// 指定されたレベルで全てのヒントを返す
    fn find_all_with_level(&self, sdk: &Sudoku, level: HintLevel) -> Vec<Hint>;
}

/// ヒントの強さレベル
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HintLevel {
    /// 最も弱いヒント（注目すべき数字やユニットのみ）
    Weak,
    /// 完全なヒント（具体的なセルと数字を指定）
    Full,
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

/// ビットマスクが1つのbitだけ立っているかチェック
pub fn is_single(mask: Mask) -> bool {
    mask.count_ones() == 1
}

/// 1つのbitだけ立っているマスクから数字を取得
pub fn single_digit(mask: Mask) -> Option<u8> {
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
pub fn bit(digit: u8) -> Mask {
    1u16 << digit
}
