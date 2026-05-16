//! End-to-end smoke test for the full `FoundationModels` wrapper surface.
//!
//! Run with: `cargo run --example 06_smoke --features macos_26_0`

use foundation_models::prelude::*;
use serde::Deserialize;
use std::io::Write;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[derive(Debug, Deserialize)]
struct EchoArgs {
    message: String,
}

fn make_echo_tool(schema: GenerationSchema, calls: Arc<AtomicUsize>) -> Tool {
    Tool::json(
        "echo",
        "Echo the provided message back to the model.",
        schema,
        move |args: EchoArgs| {
            calls.fetch_add(1, Ordering::SeqCst);
            println!("tool call: {}", args.message);
            Ok(args.message)
        },
    )
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "default availability: {:?}",
        SystemLanguageModel::availability()
    );
    if !SystemLanguageModel::is_available() {
        eprintln!("SKIP: FoundationModels unavailable");
        return Ok(());
    }

    let model = SystemLanguageModel::with_use_case(UseCase::General, Guardrails::Default)?;
    println!("supports en-US: {}", model.supports_locale("en-US"));
    let languages = model.supported_languages();
    println!(
        "supported languages (sample): {}",
        languages.into_iter().take(5).collect::<Vec<_>>().join(", ")
    );

    let tool_schema = GenerationSchema::from_dynamic(
        DynamicGenerationSchema::object("EchoArgs")
            .with_property(
                "message",
                DynamicGenerationProperty::new(
                    DynamicGenerationSchema::string()
                        .with_description("The message to echo back verbatim"),
                ),
            )
            .with_description("Arguments for the echo tool"),
        [],
    )?;

    let basic_session = LanguageModelSession::builder()
        .model(&model)
        .instructions("You are a concise assistant.")?
        .build()?;

    basic_session.prewarm_with_prompt("Warm up with a short greeting.")?;

    let hello = basic_session.respond("In six words or fewer, greet Rust.")?;
    println!("respond(): {hello}");

    print!("stream(): ");
    basic_session.stream_with(
        "Write a single short sentence about on-device AI.",
        GenerationOptions::new().with_sampling(SamplingMode::Greedy),
        |event| match event {
            StreamEvent::Chunk(chunk) => {
                print!("{chunk}");
                let _ = std::io::stdout().flush();
            }
            StreamEvent::Done => println!(),
            StreamEvent::Error(error) => eprintln!("\nstream error: {error}"),
            _ => {}
        },
    )?;

    let tool_calls = Arc::new(AtomicUsize::new(0));
    let echo_tool = make_echo_tool(tool_schema.clone(), Arc::clone(&tool_calls));
    let session = LanguageModelSession::builder()
        .model(&model)
        .instructions("You are a concise assistant. Prefer using tools when explicitly asked.")?
        .tool(echo_tool)
        .build()?;

    let tool_reply = session.respond_prompt_with(
        "Use the echo tool exactly once with the message 'Rust smoke test'. Reply with only the echoed message.",
        GenerationOptions::new().with_sampling(SamplingMode::Greedy),
    )?;
    println!("tool reply: {tool_reply}");
    if tool_calls.load(Ordering::SeqCst) == 0 {
        return Err("tool was not invoked".into());
    }

    let transcript = session.transcript()?;
    println!("transcript entries: {}", transcript.entries().len());
    let restored = LanguageModelSession::builder()
        .model(&model)
        .transcript(transcript)
        .tool(make_echo_tool(tool_schema, Arc::clone(&tool_calls)))
        .build()?;
    let restored_reply = restored.respond("Reply with exactly the word restored.")?;
    println!("restored reply: {restored_reply}");

    Ok(())
}
