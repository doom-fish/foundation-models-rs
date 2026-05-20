# foundation-models

Safe, idiomatic Rust bindings for Apple's [FoundationModels](https://developer.apple.com/documentation/foundationmodels) framework — the on-device large language model that ships with Apple Intelligence on macOS 26+.

## Features

- **Sessions and multi-turn chat** — create, restore, inspect, and persist `LanguageModelSession`s
- **Streaming** — text deltas and structured-generation snapshots
- **Async API** — executor-agnostic `Future` wrappers for `respond(to:)`, `respond(to:generating:)`, and adapter lifecycle (see `async_api` module)
- **Tool calling** — register Rust callbacks as `FoundationModels` `Tool`s
- **Structured generation** — JSON-schema validation, dynamic schemas, string-choice schemas, array guides, explicit nil-representation helpers, and Rust `Generable` traits
- **Structured content helpers** — typed `GenerationId`, string-backed `Decimal`, `GeneratedContentKind`, and generated-content builders with optional IDs
- **System model configuration** — availability, use cases, guardrails, locales, adapter handles, and async token counting
- **Transcript support** — typed transcript inspection plus raw JSON round-tripping
- **Response / tool definitions** — `ResponseFormat::generating`, inferred `Tool::generable`, and transcript `ToolDefinition` helpers
- **Typed error metadata** — `FMError` accessors for recovery suggestions, refusal helpers, tool-call details, and generation/schema/adapter-error contexts
- **Feedback attachments** — full `LanguageModelFeedback` issue/sentiment support

## Requirements

- macOS 26.0 or newer (build host **and** runtime)
- Xcode 26 SDK
- Apple Intelligence enabled in System Settings
- Apple Silicon

## Installation

```toml
[dependencies]
foundation-models = { version = "0.11.2", features = ["macos_26_0"] }
```

## Async API

Enable the `async` feature to get executor-agnostic `Future` wrappers that
work with any async runtime (Tokio, async-std, smol, pollster, …):

```toml
[dependencies]
foundation-models = { version = "0.11.2", features = ["macos_26_0", "async"] }
```

```rust,no_run
use foundation_models::{LanguageModelSession, SystemLanguageModel};
#[cfg(feature = "async")]
use foundation_models::async_api::AsyncSession;

# fn main() -> Result<(), Box<dyn std::error::Error>> {
if !SystemLanguageModel::is_available() { return Ok(()); }
#[cfg(feature = "async")]
pollster::block_on(async {
    let session = LanguageModelSession::new();
    let reply = AsyncSession::new(&session).respond("Name three Norse gods.")?.await?;
    println!("{}", reply.content);
    Ok::<(), Box<dyn std::error::Error>>(())
})?;
# Ok(())
# }
```

| Type | Apple API |
|------|-----------|
| `AsyncSession::respond` | `LanguageModelSession.respond(to:)` |
| `AsyncSession::respond_generating` | `LanguageModelSession.respond(to:generating:)` |
| `AsyncAdapter::from_name` | `SystemLanguageModel.Adapter init(name:)` |
| `AsyncAdapter::compatibility` | `Adapter.compatibleAdapterIdentifiers(name:)` |
| `AsyncAdapter::compile` | `SystemLanguageModel.Adapter.compile()` |
| `SystemLanguageModel::token_count` | `SystemLanguageModel.tokenCount(for:)` |

> **Tier 2 note:** `LanguageModelSession.streamResponse(to:)` is an `AsyncSequence`
> (multi-fire stream). It is deferred to Tier 2. Use `LanguageModelSession::stream`
> for synchronous streaming today.

## Quick start

```rust,no_run
use foundation_models::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if !SystemLanguageModel::is_available() {
        eprintln!("Unavailable: {:?}", SystemLanguageModel::availability());
        return Ok(());
    }

    let session = LanguageModelSession::with_instructions(
        "Answer in a single concise sentence.",
    );
    let reply = session.respond("Why is the sky blue?")?;
    println!("{reply}");
    Ok(())
}
```

## Tool calling

```rust,no_run
use foundation_models::prelude::*;
use serde::Deserialize;

#[derive(Deserialize)]
struct EchoArgs {
    message: String,
}

# fn main() -> Result<(), Box<dyn std::error::Error>> {
let schema = GenerationSchema::from_dynamic(
    DynamicGenerationSchema::object("EchoArgs").with_property(
        "message",
        DynamicGenerationProperty::new(DynamicGenerationSchema::string()),
    ),
    [],
)?;

let tool = Tool::json("echo", "Echo the provided message.", schema, |args: EchoArgs| {
    Ok(args.message)
});

let session = LanguageModelSession::builder()
    .instructions("Use tools when explicitly asked.")?
    .tool(tool)
    .build()?;

let reply = session.respond_prompt(
    "Use the echo tool exactly once with the message 'hello from Rust'.",
)?;
println!("{reply}");
# Ok(())
# }
```

## Structured generation

```rust,no_run
use foundation_models::prelude::*;

# fn main() -> Result<(), Box<dyn std::error::Error>> {
let schema = GenerationSchema::from_dynamic(
    DynamicGenerationSchema::object("Movie")
        .with_property(
            "title",
            DynamicGenerationProperty::new(DynamicGenerationSchema::string()),
        )
        .with_property(
            "year",
            DynamicGenerationProperty::new(DynamicGenerationSchema::integer()),
        ),
    [],
)?;

let session = LanguageModelSession::new();
let response = session.respond_generated(
    "Return JSON for one classic science-fiction movie.",
    &schema,
    true,
)?;
println!("{}", response.json_string()?);
# Ok(())
# }
```

## Coverage-oriented helpers

```rust,no_run
use foundation_models::prelude::*;

# fn main() -> Result<(), Box<dyn std::error::Error>> {
let schema = GenerationSchema::from_dynamic(
    DynamicGenerationSchema::array_of(DynamicGenerationSchema::string()).with_guides([
        GenerationGuide::minimum_count(1),
        GenerationGuide::maximum_count(3),
        GenerationGuide::element(GenerationGuide::string_pattern("^[a-z]+$")),
    ]),
    [],
)?;

let response_format = ResponseFormat::generating::<GeneratedContent>()?;
let tool = Tool::generable("echo", "Echo structured content", |args: GeneratedContent| {
    Ok(args.json_string()?)
})?;

println!("{}", schema.json_schema());
println!("{}", response_format.name());
println!("{}", tool.definition().name);
# Ok(())
# }
```

## Smoke example

```bash
cargo run --example 06_smoke --features macos_26_0
cargo run --example 07_schema_surface --features macos_26_0
```

## Notes

- Swift-only compile-time macros such as `@Generable` and `@Guide` are exposed as Rust runtime traits/builders (`Generable`, `GenerationGuide`, `DynamicGenerationSchema`).
- The previously audited `SystemLanguageModel.Adapter::isCompatible(_ assetPack:)` exemption has been removed: Xcode 26.5's `FoundationModels.swiftinterface` no longer exposes that symbol. Enable the optional `backgroundassets` feature to pull in the sibling `backgroundassets` crate for future interop helpers.
- `GenerationID` now round-trips as `GenerationId` via `GeneratedContent::generation_id_handle()`; `GeneratedContent::generation_id()` remains as a best-effort string helper.
- Typed generation/schema/adapter error metadata plus refusal helpers are available through `FMError::{generation_error_context, adapter_asset_error_context, schema_error_context, recovery_suggestion, failure_reason, refusal, tool_call_error}`.
- Xcode 26.5's `FoundationModels.swiftinterface` does **not** expose standalone `PromptTag`, `Conversation`, `ToolCallingMode`, `SystemPrompt`, `Examples`, `LanguageModelInputContent`, `LanguageModelOutputContent`, or `Streaming` symbols; see [`COVERAGE.md`](COVERAGE.md) for the audited matrix.

## License

Licensed under either of [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT) at your option.
