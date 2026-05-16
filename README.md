# foundation-models

Safe, idiomatic Rust bindings for Apple's [FoundationModels](https://developer.apple.com/documentation/foundationmodels) framework — the on-device large language model that ships with Apple Intelligence on macOS 26+.

## Features

- **Sessions and multi-turn chat** — create, restore, inspect, and persist `LanguageModelSession`s
- **Streaming** — text deltas and structured-generation snapshots
- **Tool calling** — register Rust callbacks as `FoundationModels` `Tool`s
- **Structured generation** — JSON-schema validation, dynamic schemas, and Rust `Generable` traits
- **System model configuration** — availability, use cases, guardrails, locales, and adapter handles
- **Transcript support** — typed transcript inspection plus raw JSON round-tripping
- **Feedback attachments** — full `LanguageModelFeedback` issue/sentiment support

## Requirements

- macOS 26.0 or newer (build host **and** runtime)
- Xcode 26 SDK
- Apple Intelligence enabled in System Settings
- Apple Silicon

## Installation

```toml
[dependencies]
foundation-models = { version = "0.6.0", features = ["macos_26_0"] }
```

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

## Smoke example

```bash
cargo run --example 06_smoke --features macos_26_0
```

## Notes

- Swift-only compile-time macros such as `@Generable` and `@Guide` are exposed as Rust runtime traits/builders (`Generable`, `GenerationGuide`, `DynamicGenerationSchema`).
- `SystemLanguageModel.Adapter::isCompatible(_ assetPack:)` is not wrapped because it depends on `BackgroundAssets.AssetPack`, which this crate does not expose.
- `GenerationID` remains opaque in the Apple SDK; generated-content IDs are surfaced as best-effort string metadata.

## License

Licensed under either of [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT) at your option.
