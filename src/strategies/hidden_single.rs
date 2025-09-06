use crate::strategy::{bit, is_single, Hint, Strategy};
use crate::sudoku::Sudoku;

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
        for (unit_index, unit) in sdk.units().iter().enumerate() {
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
                    if !is_single(sdk.cell_mask(target_cell)) {
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
            if sdk.cell_mask(cell_index) & digit_mask != 0 {
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
