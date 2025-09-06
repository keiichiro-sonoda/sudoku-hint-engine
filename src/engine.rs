use crate::strategies::{HiddenSingle, NakedSingle};
use crate::strategy::{Hint, Strategy};
use crate::sudoku::Sudoku;

/// 数独解法エンジン
pub struct Engine {
    strategies: Vec<Box<dyn Strategy>>,
}

impl Engine {
    /// 基本的な手筋を使うエンジンを作成
    /// 優先順位: Naked Single -> Hidden Single
    /// （まず最も簡単なNaked Singleで1手ずつ埋め、無ければ次の手筋へ）
    pub fn basic() -> Self {
        Self {
            strategies: vec![Box::new(NakedSingle), Box::new(HiddenSingle)],
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

    /// Strategy名も併せて次のヒントを返す
    pub fn next_hint_with_name(&self, sdk: &Sudoku) -> Option<(String, Hint)> {
        for s in &self.strategies {
            if let Some(h) = s.find(sdk) {
                return Some((s.name().to_string(), h));
            }
        }
        None
    }

    /// 次のヒントを1つ適用（成功時は (Strategy名, 適用したヒント) を返す）
    pub fn apply_next(&self, sdk: &mut Sudoku) -> Option<(String, Hint)> {
        let (name, hint) = self.next_hint_with_name(sdk)?;
        let cloned = hint.clone();
        // applyはselfを消費するので、複製した方を返す
        match hint.apply(sdk) {
            Ok(()) => Some((name, cloned)),
            Err(()) => None,
        }
    }

    /// 1つ埋めて次のヒント…を繰り返し、解けなくなるまで進める
    /// 戻り値: 適用したヒントの(Strategy名, ヒント)のリスト（適用順）
    pub fn solve_stepwise(
        &self,
        sdk: &mut Sudoku,
        max_steps: Option<usize>,
    ) -> Vec<(String, Hint)> {
        let mut applied = Vec::new();
        while max_steps.map(|m| applied.len() < m).unwrap_or(true) {
            if let Some((name, hint)) = self.apply_next(sdk) {
                applied.push((name, hint));
            } else {
                break;
            }
        }
        applied
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
