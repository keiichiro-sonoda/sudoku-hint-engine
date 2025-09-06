use crate::strategy::{is_single, single_digit, Hint, Strategy};
use crate::sudoku::Sudoku;

/// Naked Single: 候補が1つのセルは確定
///
/// ヒント内容: 「あるマスに注目して、このマスにはこれしか入らない」
///
/// 人が頭の中でやっていること（直感的な説明）
/// - 1つのマスを見て、そのマスに入れられる数字の候補（1〜9）を考える。
/// - 候補が1個しか残っていなければ、そのマスはその数字で"確定"。
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
            // 既に確定済みのセルは除外（初期・推論を問わず）
            if !sdk.is_confirmed(i) && is_single(sdk.cell_mask(i)) {
                let d = single_digit(sdk.cell_mask(i)).unwrap();
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
