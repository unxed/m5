//! m5: Midnight Commander-like file manager on Rust.

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    // Check for --key-test mode
    if args.len() > 1 && args[1] == "--key-test" {
        let negotiate = args.iter().skip(2).any(|a| a == "--negotiate");
        let run = if negotiate {
            m5_term::key_test::run_key_test_negotiating
        } else {
            m5_term::key_test::run_key_test
        };
        match run() {
            Ok(()) => std::process::exit(0),
            Err(e) => {
                eprintln!("Error in key test mode: {}", e);
                std::process::exit(1);
            }
        }
    }

    // Normal mode: print version
    let version = env!("CARGO_PKG_VERSION");
    println!("m5 {}", version);
    println!("Midnight Commander on Rust");
    println!();
    println!("Usage:");
    println!("  m5              Start the file manager");
    println!("  m5 --key-test   Test key input (interactive)");
    println!("  m5 --key-test --negotiate");
    println!("                  Same, after negotiating terminal abilities");
}
