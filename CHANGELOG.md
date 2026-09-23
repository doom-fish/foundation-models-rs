# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.12.0] - Unreleased

### Security

- Tool calls could dereference a freed registry: Swift's `RustTool` held a non-owning pointer to the Rust `ToolRegistry`, while detached Tasks kept the Swift session alive. `let f = AsyncSession::new(&s).respond(..)?; drop(s); f.await`, or a stream that returned early after a callback panic, let the model call a tool through freed memory. The Swift session now owns a reference to the registry and releases it in `deinit`.
- A tool error message containing a NUL byte panicked inside `extern "C"` and aborted the process. NUL bytes are replaced with U+FFFD, and every callback body now runs under `doom_fish_utils::panic_safe`.
- `Adapter::from_file` force-unwrapped `URL(string:)`, so a `file:` path that Foundation can't parse (for example `file://host:port/x`) crashed the host. It now returns an error.

### Fixed

- Error metadata (recovery suggestion, failure reason, refusal, tool-call error and the generation, schema and adapter contexts) lived in a global map keyed by the message's heap address. Entries were never removed, and a reused address made an unrelated error report another request's metadata. The metadata now travels inside the error value.
- Nothing could be cancelled. Dropping a future, or a sync stream that stops early, now cancels its Swift Task, and the next request on the session waits for cancelled requests to finish instead of failing with `ConcurrentRequests`. A cancelled stream's Task also waits (up to 2 s) for FoundationModels to roll the transcript back, because a request issued before that rollback finishes traps inside the framework (seen on macOS 27.0). `FMError::Cancelled` is now reported.
- Text deltas were computed on grapheme clusters. When a cluster grew across snapshots (ZWJ emoji, skin tones, VS16, combining marks) the whole reply was re-sent or scalars were dropped. Deltas are now byte-prefix differences of full snapshots, so concatenating chunks equals the final text.
- Async errors all became `FMError::Unknown`, `AdapterInvalidName` or `AdapterCompatibleNotFound` strings. Async operations now keep the SDK's typed error and its metadata.
- Tool handlers ran on Swift's cooperative thread pool, so a blocking handler or a nested sync `respond()` could starve it. They now run on a dispatch queue.
- `SamplingMode::TopK(0)` and top-p thresholds outside `0.0..=1.0` reached the SDK; they are now rejected with `FMError::InvalidArgument`.
- `LanguageModelSession::with_instructions` panicked with "FoundationModels is not available" when the instructions contained a NUL byte. Instructions now go through the JSON bridge, so NUL bytes are passed through.
- `LanguageModelSession::log_feedback` discarded the feedback attachment data.
- The refusal-explanation thunks read their C strings inside the detached Task; they now copy them first.
- The Swift bridge declared macOS 13 although FoundationModels is linked strongly and the README requires macOS 26. It now deploys to macOS 26; the 26.4 APIs keep their runtime checks.
- `cargo clippy -- -D warnings` failed on the current toolchain (`borrow_as_ptr`).
- The explicit-nil schema test failed on macOS 27.0, whose schema encoding no longer marks explicitly-nil properties as required; the encoding checks now run on macOS 26 only.

### Changed

- **Breaking:** `FMError` variants carry an `ErrorMessage` (text plus metadata) instead of a `String`. It derefs to `str`, displays like the old string and converts from `String` and `&str`, so construct errors with `"...".into()` or `format!(...).into()`.
- **Breaking:** `LanguageModelSession::log_feedback` returns `Result<Vec<u8>, FMError>` with the attachment data.
- **Breaking:** `ffi`: `fm_session_create_ex` takes a release callback for the tool context, which Swift always consumes; the respond, stream, compile, refusal-explanation and async exports return a task handle to cancel with `fm_task_cancel` and release with `fm_object_release`; the async adapter exports use status-carrying callbacks (`FmObjectCallback`, `FmRespondCallback`).
- `SystemLanguageModel::token_count` and `ConfiguredSystemLanguageModel::token_count` accept any `ToPrompt` (a `&str` still works).
- Dropping a `LanguageModelSession` while a future is pending is supported; the future completes.
- doom-fish-utils is a regular dependency with the requirement `>=0.4.1, <0.5`, the `backgroundassets` requirement is `>=0.4, <0.5`, and `rust-version` is 1.82.

### Deprecated

- The `backgroundassets` feature and its `foundation_models::backgroundassets` module. It only re-exports the sibling crate; its one SDK link, `Adapter.isCompatible(_ assetPack:)`, was deprecated in macOS 26.4 and removed in 27.0. Depend on `backgroundassets` directly.

### Added

- `SystemLanguageModel::context_size` and `ConfiguredSystemLanguageModel::context_size` (`contextSize`).
- `ConfiguredSystemLanguageModel::{token_count_for_instructions, token_count_for_tools, token_count_for_schema, token_count_for_transcript}` for the remaining `tokenCount(for:)` overloads (macOS 26.4+).
- `StreamEvent::Replace`, delivered when the model rewrites text it already streamed.
- `ErrorMessage`, re-exported from the crate root and the prelude.
- Regression tests for tool-registry ownership, NUL-safe tool errors, panicking callbacks, stream-state lifetimes, grapheme-cluster deltas, typed async errors, sampling validation, cancellation, and a future that outlives its session.

### Removed

- The unused legacy `ffi` exports `fm_session_create`, `fm_session_respond`, `fm_session_stream_response`, `fm_session_log_feedback` and `fm_system_model_token_count_prompt_async`, and the `FmAsyncCallback` type.
- The stray `ctk_probe.swift` and the empty `FoundationModelsBridge.h` header.

## [0.11.3] - 2026-06-06

### Fixed

- The streaming trampolines freed the stream state on the first event; it now stays alive until Swift's terminal callback.

## [0.11.2] - 2026-05-20

- Phase 32 completeness + async sweep.
- Added `AsyncAdapter::compile` for `SystemLanguageModel.Adapter.compile()` and fixed the feature-gated README async example/doctest.

## [0.11.1] - 2026-05-20

- Clippy hygiene sweep: cleared all `-D warnings` lints across the crate. No public API change.

## [0.11.0] - 2026-05-19

### Added

- Added an optional `backgroundassets` Cargo feature that pulls in the published sibling `backgroundassets` crate for cross-framework interop.

### Changed

- Updated the macOS 26.5 audit to remove the stale `SystemLanguageModel.Adapter.isCompatible(_ assetPack:)` exemption; the current SDK no longer exposes that symbol.

## [0.10.1] - 2026-05-19

- Bump MSRV from 1.70 to 1.76 to match fleet baseline.

## [0.10.0]

### Added

- `DynamicGenerationSchema::{new_with_nil_repr, NULL, null}` now cover the macOS 26.4 explicit-nil and `.null` dynamic-schema surface.
- `GenerationSchema::{new, new_with_nil_repr}` now mirror the typed property-schema builders using a `GeneratedContent` root.
- `SystemLanguageModel::token_count` and `ConfiguredSystemLanguageModel::token_count` now expose async `tokenCount(for:)`.
- Two integration tests now cover explicit-null structured generation and positive token counts.

### Changed

- Schema bridge requests now preserve their original Swift-builder payloads so macOS 26.4 explicit-nil schemas round-trip through sessions, tools, and async helpers.
- Structured-generation request builders now suppress inline schema prompting for explicit-nil schemas to avoid the SDK's current decoding failure on that path.
- Coverage docs now target the macOS 26.5 SDK audit.

## [0.9.0]

### Added

- `AdapterAssetErrorContext` and `FMError::adapter_asset_error_context()` now expose typed `SystemLanguageModel.Adapter.AssetError.Context` metadata.
- A public API helper test now covers the adapter asset-error context constructor and accessor.

### Changed

- Adapter error payloads now carry `adapterAssetErrorContext` separately from schema errors while preserving `recoverySuggestion` metadata.
- `COVERAGE.md` and `COVERAGE_AUDIT.md` now record full audited coverage; `SystemLanguageModel.Adapter.isCompatible(_ assetPack:)` is documented as EXEMPT until a sibling `backgroundassets-rs` binding exists.

## [0.8.1]

### Fixed

- **Panic safety in stream trampolines** (`json_text_stream_trampoline` and
  `structured_stream_trampoline`): user-supplied callbacks were invoked without
  `catch_unwind`, which would allow a panic to unwind across the FFI boundary
  (undefined behaviour). Each callback invocation is now wrapped with
  `std::panic::catch_unwind(AssertUnwindSafe(...))`. A mid-stream callback panic
  now terminates the stream cleanly (sends an `FMError::Unknown` to `done_tx`)
  instead of crossing the C boundary.
- **SAFETY comments** added to all `unsafe extern "C"` trampoline functions
  (`respond_trampoline`, `json_text_stream_trampoline`,
  `structured_stream_trampoline`, `schema_callback_trampoline`,
  `adapter_compile_trampoline`, `tool_callback_trampoline`) documenting the
  pointer provenance and lifetime invariants.
- **`doom-fish-utils` version range** widened from `"0.1"` to `">=0.1, <0.3"`
  to allow the next minor release without a breaking lockfile bump.

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
