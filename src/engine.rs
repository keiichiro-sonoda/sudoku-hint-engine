use crate::strategies::{HiddenSingle, NakedSingle};
use crate::strategy::{Hint, Strategy};
use crate::sudoku::Sudoku;

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
