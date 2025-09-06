mod engine;
mod strategies;
mod strategy;
mod sudoku;

use engine::Engine;
use std::env;
use sudoku::Sudoku;

fn main() {
    // 簡易CLI: 第1引数でモード指定
    // 例) `cargo run -- solve-basic` → Naked→Hidden の順で1手ずつ適用し尽くす
    let args: Vec<String> = env::args().collect();

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
    let mut sdk = Sudoku::from_string(p);
    let engine = Engine::basic();

    match args.get(1).map(|s| s.as_str()) {
        Some("solve-basic") => {
            println!("モード: solve-basic (Naked→Hiddenで1手ずつ)");
            println!("初期盤面:\n{}", p);
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
            if steps == 0 { println!("(適用できるヒントがありませんでした)"); }

            println!("\n最終候補状態:");
            sdk.display();
        }
        _ => {
            // デフォルトはデモ出力
            println!("デモ: まず1手だけ次のヒントを表示");
            println!("初期盤面:\n{}", p);
            println!("\n候補状態:");
            sdk.display();

            if let Some((name, hint)) = engine.next_hint_with_name(&sdk) {
                println!("\n次のヒント [{}]: {}", name, hint.description);
            } else {
                println!("\n利用可能なヒントはありません");
            }
        }
    }
}
