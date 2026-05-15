# foundation-models

Safe, idiomatic Rust bindings for Apple's [FoundationModels](https://developer.apple.com/documentation/foundationmodels) framework — the on-device large language model that ships with Apple Intelligence on macOS 26.0+.

> **Status:** experimental. API surface will change as more of FoundationModels is wrapped (tools, structured generation, transcripts).

## Features

- **On-device LLM** — runs entirely locally on Apple Silicon
- **Streaming generation** — token-by-token deltas via callback
- **Custom instructions** — system-prompt style guidance
- **Generation options** — temperature, max tokens, sampling modes
- **Zero dependencies** — no `objc2`, no `core-foundation`, no procedural macros
- **Async optional** — opt-in `async` feature for runtime-agnostic awaiting

## Requirements

- macOS 26.0 or newer (build host **and** runtime)
- Xcode 26 SDK (the crate's `build.rs` detects this via `xcrun --sdk macosx --show-sdk-version`)
- Apple Intelligence enabled in System Settings
- Apple Silicon (Intel Macs are not eligible)

## Installation

```toml
[dependencies]
foundation-models = { version = "0.1", features = ["macos_26_0"] }
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
        "You answer in a single concise sentence."
    );

    let reply = session.respond("Why is the sky blue?")?;
    println!("{reply}");

    Ok(())
}
```

## Streaming

```rust,no_run
use foundation_models::prelude::*;
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let session = LanguageModelSession::new();

    session.stream("Write a haiku about Rust.", |event| match event {
        StreamEvent::Chunk(s) => {
            print!("{s}");
            std::io::stdout().flush().ok();
        }
        StreamEvent::Done => println!(),
        StreamEvent::Error(e) => eprintln!("\nerror: {e}"),
        _ => {}
    })?;

    Ok(())
}
```

## Generation options

```rust,no_run
use foundation_models::prelude::*;

# fn main() -> Result<(), Box<dyn std::error::Error>> {
let opts = GenerationOptions::new()
    .with_temperature(0.7)
    .with_maximum_response_tokens(500)
    .with_sampling(SamplingMode::TopP(0.9));

let session = LanguageModelSession::new();
let reply = session.respond_with("Suggest a recipe.", opts)?;
println!("{reply}");
# Ok(())
# }
```

## Architecture

This crate uses the same Swift-bridge pattern as [screencapturekit-rs](https://github.com/doom-fish/screencapturekit-rs):

```text
┌────────────────────────────────────────────────────────────┐
│ Safe Rust API (LanguageModelSession, GenerationOptions)    │
├────────────────────────────────────────────────────────────┤
│ extern "C" FFI declarations (src/ffi/mod.rs)               │
├────────────────────────────────────────────────────────────┤
│ Swift @_cdecl bridge (swift-bridge/Sources/...)            │
├────────────────────────────────────────────────────────────┤
│ Apple FoundationModels.framework (Swift, async throws)     │
└────────────────────────────────────────────────────────────┘
```

The Swift layer hides the `async throws` surface behind callback-based C functions, so the Rust side stays dependency-free.

## Roadmap

- [ ] `Tool` protocol bridging (function calling)
- [ ] Structured generation via `Generable` (currently Swift-macro-only)
- [ ] `Transcript` inspection (per-turn token counts, attachments)
- [ ] Adapter support
- [ ] Vision-modality input once Apple ships it

## License

Licensed under either of [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT) at your option.
