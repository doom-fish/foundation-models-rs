//! Async usage via tokio's blocking pool.
//!
//! The crate's blocking API is safe to call from async contexts as long as
//! you offload the call to a blocking pool — Tokio's `spawn_blocking` is the
//! canonical pattern.
//!
//! Run with: `cargo run --example 05_async --features "macos_26_0 async"`

use foundation_models::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if !SystemLanguageModel::is_available() {
        eprintln!("SKIP: FoundationModels unavailable");
        return Ok(());
    }

    let reply = tokio::task::spawn_blocking(|| {
        let session = LanguageModelSession::new();
        session.respond("Say hi from async Rust.")
    })
    .await??;

    println!("{reply}");
    Ok(())
}
