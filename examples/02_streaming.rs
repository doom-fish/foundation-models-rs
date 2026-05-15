//! Stream a longer response token-by-token.
//!
//! Run with: `cargo run --example 02_streaming --features macos_26_0`

use foundation_models::prelude::*;
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if !SystemLanguageModel::is_available() {
        eprintln!(
            "SKIP: FoundationModels unavailable: {:?}",
            SystemLanguageModel::availability()
        );
        return Ok(());
    }

    let session = LanguageModelSession::new();
    session.stream(
        "Write a short haiku about a Rust crab named Ferris.",
        |event| match event {
            StreamEvent::Chunk(s) => {
                print!("{s}");
                std::io::stdout().flush().ok();
            }
            StreamEvent::Done => println!(),
            StreamEvent::Error(e) => eprintln!("\nstream error: {e}"),
            _ => {}
        },
    )?;
    Ok(())
}
