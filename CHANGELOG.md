# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
