//! Show how generation options affect output.
//!
//! Run with: `cargo run --example 04_options --features macos_26_0`

use foundation_models::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if !SystemLanguageModel::is_available() {
        eprintln!("SKIP: FoundationModels unavailable");
        return Ok(());
    }

    let session = LanguageModelSession::new();

    let prompt = "Suggest a creative name for a cat.";

    let deterministic = GenerationOptions::new()
        .with_sampling(SamplingMode::Greedy)
        .with_maximum_response_tokens(40);
    println!(
        "[greedy]   {}",
        session.respond_with(prompt, deterministic)?
    );

    let creative = GenerationOptions::new()
        .with_temperature(1.2)
        .with_sampling(SamplingMode::TopP(0.9))
        .with_maximum_response_tokens(40);
    println!("[creative] {}", session.respond_with(prompt, creative)?);

    Ok(())
}
