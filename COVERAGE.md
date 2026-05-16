# FoundationModels SDK coverage (macOS 26.2 / Xcode 26.2)

Audited against:

- `System/Library/Frameworks/FoundationModels.framework/Versions/A/Modules/FoundationModels.swiftmodule/arm64e-apple-macos.swiftinterface`
- Rust surface under `src/`
- Swift bridge under `swift-bridge/Sources/FoundationModelsBridge/`

## Requested surface matrix

| SDK symbol / area | Status | Rust / bridge coverage | Notes |
| --- | --- | --- | --- |
| `LanguageModelSession` | ✅ implemented | `src/session/mod.rs`, `swift-bridge/Sources/FoundationModelsBridge/FoundationModels.swift`, `SessionExtras.swift` | Covers session construction, transcript restore/export, `respond`, structured generation, streaming, tool calling, feedback attachments, `prewarm(promptPrefix:)`, and `isResponding`. |
| `SystemLanguageModel` | ✅ implemented | `src/model/mod.rs`, `swift-bridge/Sources/FoundationModelsBridge/ModelBridge.swift` | Covers availability, use cases, guardrails, configured handles, locale support, and adapters. |
| `Tool` | ✅ implemented | `src/tool.rs`, `swift-bridge/Sources/FoundationModelsBridge/ToolsBridge.swift` | Rust exposes dynamic tools plus schema-inferred `Tool::generable`; Swift bridge materializes them as FoundationModels `Tool`s. |
| `ToolCall` | ✅ implemented | `src/transcript.rs` | Covered as `foundation_models::ToolCall` / `Transcript::ToolCalls`. |
| `Transcript` | ✅ implemented | `src/transcript.rs`, `swift-bridge/Sources/FoundationModelsBridge/BridgeJSON.swift` | Covers transcript round-tripping, typed entries, collection helpers, and session restoration. |
| `TranscriptEntry` | ✅ implemented | `src/transcript.rs` | Re-exported as `TranscriptEntry` (`Entry` enum) with typed variants and `id()` helper. |
| `GenerationOptions` | ✅ implemented | `src/generation/mod.rs`, `swift-bridge/Sources/FoundationModelsBridge/FoundationModels.swift`, `BridgeJSON.swift` | Covers temperature, max tokens, greedy / top-k / top-p sampling, and deterministic seeds. |
| `GenerationSchema` | ✅ implemented | `src/schema.rs`, `swift-bridge/Sources/FoundationModelsBridge/SchemaBridge.swift` | Covers validated schemas, dynamic schemas, string-choice unions, and Swift-backed compilation. |
| `GenerationGuide` | ✅ implemented | `src/schema.rs`, `swift-bridge/Sources/FoundationModelsBridge/SchemaBridge.swift` | Covers string / numeric / decimal guides plus array count / element guides. |
| `Generable` | ✅ implemented | `src/schema.rs`, `src/content.rs` | Rust trait mirrors the SDK protocol and covers primitives, arrays, options, and `GeneratedContent`. |
| `Instructions` | ✅ implemented | `src/prompt.rs`, `swift-bridge/Sources/FoundationModelsBridge/BridgeJSON.swift` | Covers text + structured instructions and system-instructions session builders. |
| `Prompt` | ✅ implemented | `src/prompt.rs`, `swift-bridge/Sources/FoundationModelsBridge/BridgeJSON.swift` | Covers text + structured prompts, response requests, and prewarm prompt prefixes. |
| `PromptTag` | ⏭️ skipped | n/a | No standalone public symbol in the macOS 26.2 `FoundationModels.swiftinterface`. |
| `LanguageModelInputContent` | ⏭️ skipped | n/a | No standalone public symbol in the macOS 26.2 `FoundationModels.swiftinterface`. |
| `LanguageModelOutputContent` | ⏭️ skipped | n/a | No standalone public symbol in the macOS 26.2 `FoundationModels.swiftinterface`. |
| `ResponseFormat` | ✅ implemented | `src/prompt.rs`, `src/transcript.rs` | Covers schema-backed response formats plus `ResponseFormat::generating`. |
| `Conversation` | ⏭️ skipped | n/a | No standalone public symbol in the macOS 26.2 `FoundationModels.swiftinterface`; transcript/session history is represented by `Transcript`. |
| `ToolDefinition` | ✅ implemented | `src/prompt.rs`, `src/tool.rs`, `src/transcript.rs` | Covers explicit definitions plus conversion from `Tool` / `ToolSpec`. |
| `ToolCallingMode` | ⏭️ skipped | n/a | No standalone public symbol in the macOS 26.2 `FoundationModels.swiftinterface`. |
| `SystemPrompt` | ⏭️ skipped | n/a | No standalone public symbol in the macOS 26.2 `FoundationModels.swiftinterface`; system prompting is represented by `Instructions`. |
| `Examples` | ⏭️ skipped | n/a | No standalone public symbol in the macOS 26.2 `FoundationModels.swiftinterface`. |
| `Streaming` | ✅ implemented | `src/session/mod.rs` | Covered by `stream`, `stream_prompt`, `stream_generated`, `StreamEvent`, and `StructuredStreamEvent`. The SDK does not expose a standalone `Streaming` type. |

## Additional audited SDK items

| SDK symbol | Status | Notes |
| --- | --- | --- |
| `PromptBuilder` / `InstructionsBuilder` | ⏭️ skipped | Swift-only function-builder syntax; Rust uses `Prompt`, `Instructions`, `ToPrompt`, and `ToInstructions`. |
| `@Generable` / `@Guide` macros | ⏭️ skipped | Swift compile-time macros; Rust exposes runtime `Generable`, `GenerationGuide`, and `DynamicGenerationSchema` builders instead. |
| `SystemLanguageModel.Adapter.isCompatible(_ assetPack:)` | ⏭️ skipped | Depends on `BackgroundAssets.AssetPack`, which this crate does not expose. |
| `GenerationID` | 🟡 partial | Exposed as best-effort string metadata on `GeneratedContent::generation_id()` rather than an opaque Rust handle. |

## Verification

- `cargo test`
- `cargo test --features macos_26_0`
- `cargo clippy --all-features --all-targets -- -D warnings`
- `for ex in 01_hello 02_streaming 03_instructions 04_options 05_async 06_smoke 07_schema_surface; do cargo run --example "$ex" --features "macos_26_0 async"; done`
