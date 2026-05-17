# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.8.0]

### Added

- **`async_api` module** (Tier-1 async, gated on `async` Cargo feature): executor-agnostic
  `Future` newtypes wrapping the Apple `async throws` surface of `FoundationModels`.
  Works with any async runtime (Tokio, async-std, smol, pollster, …).

  | Rust type | Apple API |
  |-----------|-----------|
  | `AsyncSession::respond` | `LanguageModelSession.respond(to:)` |
  | `AsyncSession::respond_with_options` | `LanguageModelSession.respond(to:)` with custom `GenerationOptions` |
  | `AsyncSession::respond_generating` | `LanguageModelSession.respond(to:generating:)` |
  | `AsyncAdapter::from_name` | `SystemLanguageModel.Adapter init(name:)` |
  | `AsyncAdapter::compatibility` | `Adapter.compatibleAdapterIdentifiers(name:)` |

  `LanguageModelSession.streamResponse(to:)` is an `AsyncSequence` (multi-fire stream)
  and is deferred to **Tier 2**.

- New Swift `@_cdecl` thunks in `swift-bridge/Sources/FoundationModelsBridge/Async.swift`:
  `fm_adapter_create_from_name_async` and `fm_adapter_compatibility_async`.
- Matching `extern "C"` declarations and `FmAsyncCallback` type alias in `src/ffi/mod.rs`.
- New examples: `examples/08_async_respond.rs` and `examples/09_async_adapter.rs`.
- New integration tests in `tests/async_api_tests.rs` (7 tests: happy paths + error paths
  for each Future type, plus NUL-byte validation guards).
- `doom-fish-utils` added as an optional dependency (pulled in by the `async` feature).
- `pollster = "0.3"` added to `dev-dependencies`.



### Changed

- Adapter error surface now fully bridged: `SystemLanguageModel.Adapter.AssetError.Context` (with `debugDescription` property and `init` constructor) and `recoverySuggestion` property are now extracted and preserved across the FFI boundary via the `assetErrorPayload()` Swift bridge function.
- `COVERAGE_AUDIT_V2.md` now reflects 100% audited coverage (0 gaps). The single remaining unadjustable symbol is `SystemLanguageModel.Adapter.isCompatible(_:)`, which depends on the external `BackgroundAssets.AssetPack` type and is documented as EXEMPT with a framework-dependency citation.

## [0.7.2]

### Added

- Five new integration tests under `tests/` now cover the model, session, transcript, tool, and generation helper surfaces separately.

## [0.7.1]

### Added

- Typed generated-content helpers: `GenerationId`, string-backed `Decimal`, `GeneratedContentKind`, and `GeneratedContent` constructors/builders that preserve optional IDs.
- Typed error metadata accessors: `GenerationErrorContext`, `SchemaErrorContext`, `Refusal`, `ToolCallError`, plus `FMError::{generation_error_context, schema_error_context, recovery_suggestion, failure_reason, refusal, tool_call_error}`.

### Changed

- Structured prompt/instructions/feedback/response bridge payloads now preserve generated-content IDs across the FFI boundary.
- `COVERAGE_AUDIT.md` now closes the `GenerationID`, `Decimal`, refusal, tool-call, and schema/generation error metadata gaps (98.1% audited coverage; 5 adapter-related gaps remain).

### Fixed

- `build.rs` now links against the macOS SDK Swift runtime stubs so `cargo test` resolves `swiftCore` / `swift_Concurrency` symbols pulled in by the Swift bridge archive.

### Notes

- `BackgroundAssets.AssetPack` and the remaining adapter asset-error metadata surface are still the only audited gaps.

## [0.7.0]

### Added

- Coverage-oriented schema helpers: `DynamicGenerationSchema::any_of_strings`, array `GenerationGuide` count/element helpers, `ResponseFormat::generating`, `Tool::generable`, and `ToolDefinition` / `ToolSpec` definition conversion.
- Additional transcript ergonomics: collection-style helpers plus constructors for transcript entry types and segments.
- A new non-model example (`examples/07_schema_surface.rs`) and feature-gated helper tests covering the new schema / tool surface.
- `COVERAGE.md`, documenting the audited FoundationModels surface and the doc-name symbols that are absent from Xcode 26.2's public swiftinterface.

### Changed

- The Swift schema bridge now understands array count / element guides emitted from Rust.
- `tests/api_coverage.rs` now asserts that the requested doc-only names are absent from the public SDK interface.

### Notes

- `PromptTag`, `Conversation`, `ToolCallingMode`, `SystemPrompt`, `Examples`, `LanguageModelInputContent`, `LanguageModelOutputContent`, and `Streaming` are not standalone public symbols in the macOS 26.2 `FoundationModels.swiftinterface`; they are tracked in `COVERAGE.md` as audited absences.

## [0.6.0]

### Added

- Full `SystemLanguageModel` configuration surface: use cases, guardrails, locale support, configured model handles, and adapter management.
- Tool calling with Rust callbacks mapped onto FoundationModels `Tool` instances.
- Typed prompt/instructions builders plus transcript parsing and transcript-based session restoration.
- Structured-generation support via `GenerationSchema`, `DynamicGenerationSchema`, `GenerationGuide`, `GeneratedContent`, and a Rust `Generable` trait.
- Detailed response metadata (`SessionResponse<T>`) and structured-stream snapshots.
- Full feedback attachment support via `LanguageModelFeedback` sentiments, issues, desired response text/content, and raw attachment bytes.
- A new end-to-end smoke example (`examples/06_smoke.rs`) that exercises respond, streaming, tool calling, and transcript restoration.

### Changed

- `GenerationOptions` now supports deterministic sampling seeds.
- The Swift bridge has been split into focused files (`BridgeJSON`, `ModelBridge`, `ToolsBridge`, `SessionExtras`, `SchemaBridge`).
- `tests/api_coverage.rs` now scans the whole Swift bridge and Rust API surface instead of a single Swift file.

### Fixed

- Updated the feedback bridge for the current SDK rename from `LanguageModelFeedbackAttachment` to `LanguageModelFeedback`.

### Notes

- Swift-only compile-time macros (`@Generable`, `@Guide`) remain represented by Rust runtime traits/builders rather than direct macro bindings.
- `SystemLanguageModel.Adapter::isCompatible(_ assetPack:)` is intentionally not wrapped because it depends on `BackgroundAssets.AssetPack`, which this crate does not expose.
- `GenerationID` remains opaque in the Apple SDK; generated-content IDs are surfaced as best-effort string metadata rather than a fully round-trippable Rust handle.
