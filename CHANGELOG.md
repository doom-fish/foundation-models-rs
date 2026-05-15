# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Initial scaffold: `SystemLanguageModel` availability query
- `LanguageModelSession::new` / `with_instructions`
- Blocking `respond` / `respond_with`
- Streaming `stream` / `stream_with` with delta-only chunks
- `GenerationOptions` (temperature, max tokens, sampling modes)
- Swift bridge with `@_cdecl` exports, callback-based async-to-sync
- Build-script SDK detection mirroring screencapturekit-rs
