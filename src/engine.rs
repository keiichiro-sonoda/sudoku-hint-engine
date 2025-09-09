use crate::strategies::{HiddenSingle, NakedSingle};
use crate::strategy::{bit, is_single, Hint, HintLevel, Strategy};
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

    /// 指定されたレベルで次に使える手筋を探してヒントを返す（最初の1つ）
    pub fn next_hint_with_level(&self, sdk: &Sudoku, level: HintLevel) -> Option<Hint> {
        for s in &self.strategies {
            if let Some(h) = s.find_with_level(sdk, level) {
                return Some(h);
            }
        }
        None
    }

    /// 指定されたレベルでStrategy名も併せて次のヒントを返す
    pub fn next_hint_with_name_and_level(&self, sdk: &Sudoku, level: HintLevel) -> Option<(String, Hint)> {
        for s in &self.strategies {
            if let Some(h) = s.find_with_level(sdk, level) {
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

    /// 現在の盤面から少なくとも1つの解が存在するかを判定（バックトラック + 伝播）
    /// 既知の答えに依存せず、一貫性（充足可能性）を検証する。
    pub fn has_solution(sdk: &Sudoku) -> bool {
        // 1) 矛盾の早期検出
        if Self::has_immediate_contradiction(sdk) {
            return false;
        }

        // 2) 全セルが単一候補 = 解が完成
        if (0..81).all(|i| is_single(sdk.cell_mask(i))) {
            return true;
        }

        // 3) 候補が最少のセルを選ぶ（MRV）
        let mut best_i: Option<usize> = None;
        let mut best_cnt: u32 = u32::MAX;
        for i in 0..81 {
            let mask = sdk.cell_mask(i);
            let cnt = mask.count_ones();
            if cnt >= 2 && cnt < best_cnt {
                best_cnt = cnt;
                best_i = Some(i);
                if cnt == 2 {
                    break;
                } // 早期に良い候補が見つかったら打ち切り
            }
        }

        let i = match best_i {
            Some(i) => i,
            None => return true,
        };

        // 4) そのセルの各候補を試行
        let mask = sdk.cell_mask(i);
        for d in 1..=9u8 {
            if mask & bit(d) != 0 {
                let mut next = sdk.clone();
                if next.assign(i, d).is_ok() {
                    if Self::has_solution(&next) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// 即時の矛盾があるか（候補ゼロ、または任意のユニットで特定数字を置く場所ゼロ）
    fn has_immediate_contradiction(sdk: &Sudoku) -> bool {
        // 候補ゼロのセル
        for i in 0..81 {
            if sdk.cell_mask(i) == 0 {
                return true;
            }
        }
        // 各ユニット x 各数字で少なくとも1セルがその数字を許容するか
        for unit in sdk.units() {
            for d in 1..=9u8 {
                let m = bit(d);
                if !unit.iter().any(|&i| sdk.cell_mask(i) & m != 0) {
                    return true;
                }
            }
        }
        false
    }

    /// 初期盤面に一連の手（セル, 数字）を適用し、途中で矛盾が出たら None、
    /// 矛盾がなければ最終状態を返す。
    fn apply_moves(initial: &Sudoku, moves: &[(usize, u8)]) -> Option<Sudoku> {
        let mut sdk = initial.clone();
        for &(i, d) in moves {
            if sdk.assign(i, d).is_err() {
                return None;
            }
        }
        Some(sdk)
    }

    /// 与えた一連のユーザー手が許容される（少なくとも1解が存在）か
    pub fn has_solution_with_moves(initial: &Sudoku, moves: &[(usize, u8)]) -> bool {
        match Self::apply_moves(initial, moves) {
            Some(sdk) => Self::has_solution(&sdk),
            None => false,
        }
    }

    /// 単独の手 (i,d) が初期盤面と両立するか（初期+その1手で解が存在するか）
    pub fn is_move_possible(initial: &Sudoku, mv: (usize, u8)) -> bool {
        Self::has_solution_with_moves(initial, &[mv])
    }

    /// 現在のユーザー手集合が UNSAT のとき、その中から「局所的最小矛盾集合」を抽出。
    /// 削除ベースで不要な手を落としていき、これ以上1手削ると SAT になる集合を返す。
    pub fn minimal_unsat_subset(initial: &Sudoku, mut moves: Vec<(usize, u8)>) -> Vec<(usize, u8)> {
        let mut changed = true;
        while changed {
            changed = false;
            let mut i = 0;
            while i < moves.len() {
                let mut tmp = moves.clone();
                tmp.remove(i);
                if !Self::has_solution_with_moves(initial, &tmp) {
                    moves = tmp;
                    changed = true;
                } else {
                    i += 1;
                }
            }
        }
        moves
    }

    /// ユーザーが早すぎるタイミングで埋めた数字を検出する（ユーザー整合推論）
    ///
    /// アルゴリズム（提案どおり）
    /// 1) 初期盤面から開始
    /// 2) 現在の状態で基本戦略（Naked/Hidden Single）の候補手を列挙
    /// 3) その中で「ユーザーが既に入れている手」だけを適用
    /// 4) 1手でも適用できたら2)へ、1手も適用できなければ停止
    /// 5) 最終的に正当化できなかったユーザー手が「早すぎる割り当て」
    pub fn find_premature_moves(initial: &Sudoku, user_moves: &[(usize, u8)]) -> Vec<(usize, u8)> {
        use std::collections::HashSet;

        let engine = Self::basic();
        let user_set: HashSet<(usize, u8)> = user_moves.iter().copied().collect();

        let mut current = initial.clone();
        let mut justified: HashSet<(usize, u8)> = HashSet::new();

        loop {
            // 候補ヒントを全て収集
            let hints = engine.all_hints(&current);
            if hints.is_empty() {
                break; // もう進めない
            }

            // ユーザーが既に入れている手に一致する割当だけを抽出
            let mut to_apply: Vec<(usize, u8)> = Vec::new();
            for (_name, hs) in &hints {
                for h in hs {
                    for &(cell, digit) in &h.assignments {
                        if user_set.contains(&(cell, digit)) && !current.is_confirmed(cell) {
                            to_apply.push((cell, digit));
                        }
                    }
                }
            }

            // 重複除去
            to_apply.sort();
            to_apply.dedup();

            if to_apply.is_empty() {
                break; // ユーザーが入れている手と整合する前進ができない
            }

            // 一致する手だけを全て適用（順不同、互いに矛盾はしない想定）
            let mut progressed = false;
            for (cell, digit) in to_apply {
                if !current.is_confirmed(cell) {
                    if current.assign(cell, digit).is_ok() {
                        justified.insert((cell, digit));
                        progressed = true;
                    }
                }
            }

            if !progressed {
                break; // 進展なし
            }
        }

        // 正当化できなかったユーザー手 = 早すぎる割り当て
        let mut premature = Vec::new();
        for &(cell, digit) in user_moves {
            if !justified.contains(&(cell, digit)) {
                premature.push((cell, digit));
            }
        }
        premature
    }

    /// 単独のユーザー手（1手）を除いて解が存在するかをチェックし、
    /// "外すと解ける" 手のリストを返す（単独ミス候補の列挙）
    pub fn suspect_single_moves(initial: &Sudoku, user_moves: &[(usize, u8)]) -> Vec<(usize, u8)> {
        let mut suspects = Vec::new();
        for (idx, &(cell, digit)) in user_moves.iter().enumerate() {
            let mut sdk = initial.clone();
            // 全ユーザー手を適用（この手だけ除外）
            for (j, &(c, d)) in user_moves.iter().enumerate() {
                if j == idx {
                    continue;
                }
                if sdk.assign(c, d).is_err() {
                    // この時点で矛盾なら、別の手との衝突だが、この候補評価では
                    // "除外手"とは無関係に既に壊れている可能性がある。継続して判定。
                }
            }
            if Self::has_solution(&sdk) {
                suspects.push((cell, digit));
            }
        }
        suspects
    }

    /// 2手の組を除いて解が存在するかをチェックし、最初に見つかった組を返す
    /// 上限 `max_pairs` で計算量を制限
    pub fn suspect_pair_move(
        initial: &Sudoku,
        user_moves: &[(usize, u8)],
        max_pairs: usize,
    ) -> Option<((usize, u8), (usize, u8))> {
        let n = user_moves.len();
        let mut tested = 0usize;
        for a in 0..n {
            for b in (a + 1)..n {
                if tested >= max_pairs {
                    return None;
                }
                tested += 1;
                let mut sdk = initial.clone();
                // 全ユーザー手を適用（a,b を除外）
                for (k, &(c, d)) in user_moves.iter().enumerate() {
                    if k == a || k == b {
                        continue;
                    }
                    if sdk.assign(c, d).is_err() {
                        // 他手の時点で矛盾
                    }
                }
                if Self::has_solution(&sdk) {
                    return Some((user_moves[a], user_moves[b]));
                }
            }
        }
        None
    }
}
