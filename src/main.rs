mod engine;
mod strategies;
mod strategy;
mod sudoku;

use engine::Engine;
use strategies::{HiddenSingle, NakedSingle};
use strategy::Strategy;
use sudoku::Sudoku;

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
