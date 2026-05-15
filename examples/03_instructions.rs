//! Demonstrate custom system instructions.
//!
//! Run with: `cargo run --example 03_instructions --features macos_26_0`

use foundation_models::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if !SystemLanguageModel::is_available() {
        eprintln!("SKIP: FoundationModels unavailable");
        return Ok(());
    }

    let session = LanguageModelSession::with_instructions(
        "You are a terse Norse skald. Answer in under 10 words, in alliterative verse.",
    );

    for question in ["What is the sea?", "What is fire?", "What is courage?"] {
        println!("Q: {question}");
        println!("A: {}\n", session.respond(question)?);
    }
    Ok(())
}
