//! Minimal hello-world: ask a single question and print the response.
//!
//! Run with: `cargo run --example 01_hello --features macos_26_0`

use foundation_models::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if !SystemLanguageModel::is_available() {
        eprintln!(
            "SKIP: FoundationModels unavailable: {:?}",
            SystemLanguageModel::availability()
        );
        return Ok(());
    }

    let session = LanguageModelSession::new();
    let reply = session.respond("In one short sentence, what is Rust?")?;
    println!("{reply}");
    Ok(())
}
