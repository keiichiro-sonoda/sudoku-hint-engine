mod engine;
mod strategies;
mod strategy;
mod sudoku;

use clap::{Parser, Subcommand};
use engine::Engine;
use std::fs;
use strategy::single_digit;
use sudoku::Sudoku;

#[derive(Parser)]
#[command(name = "sudoku-hint-engine")]
#[command(about = "A Sudoku hint engine that provides strategic advice")]
#[command(long_about = "
This tool helps solve Sudoku puzzles using basic strategies (Naked Single, Hidden Single).
It can solve puzzles step-by-step or provide advice on user progress.
")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Solve a puzzle step-by-step using basic strategies
    SolveBasic {
        /// Path to the initial board file
        #[arg(short, long)]
        board: String,
    },
    /// Provide advice on user progress
    Advise {
        /// Path to the initial board file
        #[arg(short, long)]
        initial: String,
        /// Path to the user's current board file
        #[arg(short, long)]
        user: String,
    },
    /// Show next hint for a puzzle
    Hint {
        /// Path to the board file
        #[arg(short, long)]
        board: String,
    },
}

// 81マスの盤面文字列（数字/'.'/'0'、空白改行は無視）を Option<u8> のベクタに変換
fn parse_board(s: &str) -> Vec<Option<u8>> {
    let bytes: Vec<u8> = s.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    assert!(bytes.len() == 81, "Need 81 chars (digits or '.'/'0')");
    bytes
        .into_iter()
        .map(|b| {
            if (b'1'..=b'9').contains(&b) {
                Some((b - b'0') as u8)
            } else {
                None
            }
        })
        .collect()
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::SolveBasic { board } => {
            let board_str = fs::read_to_string(&board)
                .unwrap_or_else(|_| panic!("Failed to read board file: {}", board));

            println!("モード: solve-basic (Naked→Hiddenで1手ずつ)");
            println!("盤面ファイル: {}", board);
            println!("盤面内容:\n{}", board_str.trim());

            let mut sdk = Sudoku::from_string(&board_str);
            let engine = Engine::basic();

            println!("\n開始時の候補:");
            sdk.display();

            println!("\n適用ログ:");
            let mut steps = 0usize;
            loop {
                match engine.apply_next(&mut sdk) {
                    Some((name, hint)) => {
                        steps += 1;
                        println!("{:>3}. [{}] {}", steps, name, hint.description);
                    }
                    None => break,
                }
            }

            if steps == 0 {
                println!("(適用できるヒントがありませんでした)");
            }

            println!("\n最終候補状態:");
            sdk.display();
        }

        Commands::Advise { initial, user } => {
            let init_str = fs::read_to_string(&initial)
                .unwrap_or_else(|_| panic!("Failed to read initial board file: {}", initial));
            let user_str = fs::read_to_string(&user)
                .unwrap_or_else(|_| panic!("Failed to read user board file: {}", user));

            println!("モード: advise");
            println!("初期盤面: {}", initial);
            println!("ユーザー盤面: {}", user);

            // 初期状態を基にSudokuを構築
            let mut base = Sudoku::from_string(&init_str);
            // ユーザーの状態を解析
            let user = parse_board(&user_str);

            println!("\n検証結果: 初期値の整合性をチェック中...");

            // ユーザーの確定を順に適用して検証
            let mut errors: Vec<String> = Vec::new();
            for i in 0..81 {
                if let Some(d) = user[i] {
                    let (r, c) = (i / 9 + 1, i % 9 + 1);
                    if base.initially_given(i) {
                        // 初期確定と一致するか
                        match single_digit(base.cell_mask(i)) {
                            Some(g) if g == d => { /* OK: そのまま */ }
                            Some(g) => errors.push(format!(
                                "セル({},{}) 初期値 {} とユーザー指定 {} が不一致",
                                r, c, g, d
                            )),
                            None => errors.push(format!(
                                "セル({},{}) は初期確定扱いだがマスクが単一でない",
                                r, c
                            )),
                        }
                    } else {
                        // 初期非確定マスへユーザーの数字を割当
                        if let Err(()) = base.assign(i, d) {
                            errors.push(format!(
                                "セル({},{}) に {} を置くと矛盾が発生（行/列/ブロックの制約違反）",
                                r, c, d
                            ));
                        }
                    }
                }
            }

            if !errors.is_empty() {
                println!("\n検証結果: NG");
                for e in errors {
                    println!("- {}", e);
                }
                return;
            }

            // ユーザーが置いた手（初期以外）を収集
            let user_moves: Vec<(usize, u8)> = parse_board(&user_str)
                .into_iter()
                .enumerate()
                .filter_map(|(i, od)| od.map(|d| (i, d)))
                .filter(|(i, _)| !base.initially_given(*i))
                .collect();

            // 早すぎるタイミングで埋めた数字をチェック
            let premature =
                Engine::find_premature_moves(&Sudoku::from_string(&init_str), &user_moves);
            println!("\n検証結果: OK（ここまでの進め方は矛盾なし）");

            if !premature.is_empty() {
                println!("\n基本戦略（Naked Single/Hidden Single）では、まだ確定できないはずの数字があります:");
                println!(
                    "（ユーザーが埋めた {}個中 {}個が早すぎます）",
                    user_moves.len(),
                    premature.len()
                );

                for (i, d) in premature.iter() {
                    println!(
                        "- セル({},{}) の {} はまだ確定できないはずです",
                        i / 9 + 1,
                        i % 9 + 1,
                        d
                    );
                }

                // 確定できない数字を除外した盤面状態を構築
                let premature_set: std::collections::HashSet<(usize, u8)> =
                    premature.iter().copied().collect();
                let mut corrected_board = vec![None; 81];

                // 初期盤面をコピー
                for i in 0..81 {
                    if let Some(d) = user[i] {
                        // 初期確定の場合はそのまま、ユーザーが埋めた数字で早すぎるもの以外を採用
                        if base.initially_given(i) || !premature_set.contains(&(i, d)) {
                            corrected_board[i] = Some(d);
                        }
                    }
                }

                // 除外済み盤面状態を文字列として表示
                let corrected_board_str: String = corrected_board
                    .iter()
                    .map(|cell| match cell {
                        Some(d) => char::from_digit(*d as u32, 10).unwrap_or('.'),
                        None => '.',
                    })
                    .collect();

                println!("\n早すぎる数字を除外した盤面状態:");
                for row in 0..9 {
                    let row_str: String = corrected_board_str[row * 9..(row + 1) * 9]
                        .chars()
                        .collect();
                    println!("{}", row_str);
                }

                println!("\nヒント: より基本的な手筋から順番に進めることをお勧めします。");

                // 次のアドバイス（手筋）は「早すぎる手」を除いた状態から探索する
                let mut temp_base = Sudoku::from_string(&init_str);
                for &(i, d) in &user_moves {
                    if !premature_set.contains(&(i, d)) {
                        let _ = temp_base.assign(i, d);
                    }
                }

                let eng = Engine::basic();
                if let Some((strategy_name, hint)) = eng.next_hint_with_name(&temp_base) {
                    println!(
                        "まず次の手から始めてみてください [{}]: {}",
                        strategy_name, hint.description
                    );
                } else {
                    println!("次の手筋は見つかりませんでした（早すぎる手を外した状態でも詰まっています）");
                }
                return;
            }

            if !Engine::has_solution(&base) {
                println!("\n整合性チェック: NG（この状態からは解が存在しません）");
                println!(
                    "- 現在の入れ方のどこかに潜在的な矛盾があります（即時には現れないタイプ）"
                );

                // どの手が問題かの手がかり（強い判定）:
                // 「この1手を採用した状態が解無し」= その手は確実に誤り
                let definite_wrong: Vec<(usize, u8)> = user_moves
                    .iter()
                    .copied()
                    .filter(|&(i, d)| {
                        !Engine::is_move_possible(&Sudoku::from_string(&init_str), (i, d))
                    })
                    .collect();
                if !definite_wrong.is_empty() {
                    println!("\n確実に誤っている手:");
                    for (i, d) in &definite_wrong {
                        println!("- セル({},{}) の {}", i / 9 + 1, i % 9 + 1, d);
                    }
                }

                // 弱い判定: 「この1手を外すと解が存在」= その手は矛盾に関与
                let suspects =
                    Engine::suspect_single_moves(&Sudoku::from_string(&init_str), &user_moves);
                if !suspects.is_empty() {
                    println!("\n疑わしい手（この1手を外すと解が存在）:");
                    for (i, d) in suspects {
                        println!("- セル({},{}) の {}", i / 9 + 1, i % 9 + 1, d);
                    }
                }

                // 単独で特定できない場合、2手の組み合わせでの候補を一つ示す
                if let Some(((i1, d1), (i2, d2))) =
                    Engine::suspect_pair_move(&Sudoku::from_string(&init_str), &user_moves, 500)
                {
                    println!("\n疑わしい組み合わせ（この2手を外すと解が存在）:");
                    println!("- セル({},{}) の {}", i1 / 9 + 1, i1 % 9 + 1, d1);
                    println!("- セル({},{}) の {}", i2 / 9 + 1, i2 % 9 + 1, d2);
                    println!("（他にも組がある可能性はあります）");
                } else {
                    println!(
                        "\n疑わしい単独手や2手組を特定できませんでした（複合的な矛盾の可能性）"
                    );
                }

                // 最小矛盾集合（局所的）を提示して、見直し優先範囲を絞る
                let minimal =
                    Engine::minimal_unsat_subset(&Sudoku::from_string(&init_str), user_moves);
                if !minimal.is_empty() {
                    println!("\n最小矛盾候補セット（この集合全体が同時には成立しません）:");
                    for (i, d) in minimal {
                        println!("- セル({},{}) の {}", i / 9 + 1, i % 9 + 1, d);
                    }
                }
                return;
            }

            // 次のアドバイス（手筋）を提示
            let eng = Engine::basic();
            if let Some((name, hint)) = eng.next_hint_with_name(&base) {
                println!("次のアドバイス [{}]: {}", name, hint.description);
            } else {
                println!("次のアドバイス: 現在の手筋では見つかりません（詰み/高難度手筋が必要）");
            }
        }

        Commands::Hint { board } => {
            let board_str = fs::read_to_string(&board)
                .unwrap_or_else(|_| panic!("Failed to read board file: {}", board));

            println!("モード: hint");
            println!("盤面ファイル: {}", board);

            let sdk = Sudoku::from_string(&board_str);
            let engine = Engine::basic();

            println!("\n現在の候補状態:");
            sdk.display();

            if let Some((name, hint)) = engine.next_hint_with_name(&sdk) {
                println!("\n次のヒント [{}]: {}", name, hint.description);
            } else {
                println!("\n利用可能なヒントはありません");
            }
        }
    }
}
