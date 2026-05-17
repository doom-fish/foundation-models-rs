//! Async respond example — uses `async_api::AsyncSession`.
//!
//! Demonstrates `LanguageModelSession.respond(to:)` and
//! `LanguageModelSession.respond(to:generating:)` as executor-agnostic Futures.
//!
//! Run with:
//! ```text
//! cargo run --example 08_async_respond --features "macos_26_0 async"
//! ```

use foundation_models::async_api::AsyncSession;
use foundation_models::schema::{
    DynamicGenerationProperty, DynamicGenerationSchema, GenerationSchema,
};
use foundation_models::{GenerationOptions, LanguageModelSession, SystemLanguageModel};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if !SystemLanguageModel::is_available() {
        eprintln!("SKIP: FoundationModels unavailable on this system");
        return Ok(());
    }

    pollster::block_on(async {
        let session = LanguageModelSession::new();
        let async_session = AsyncSession::new(&session);

        // --- respond(to:) ---
        let reply = async_session.respond("Name three Norse gods.")?.await?;
        println!("respond(to:): {}", reply.content);

        // --- respond(to:generating:) --- structured output ---
        let schema = DynamicGenerationSchema::object("GodList").with_property(
            "names",
            DynamicGenerationProperty::new(DynamicGenerationSchema::string())
                .with_description("comma-separated list of god names"),
        );
        let gs = GenerationSchema::from_dynamic(schema, [])?;

        let structured = async_session
            .respond_generating(
                "List three Norse gods as JSON.",
                &gs,
                true,
                GenerationOptions::new(),
            )?
            .await?;
        println!("respond(to:generating:): {:?}", structured.content);

        Ok::<(), Box<dyn std::error::Error>>(())
    })
}
