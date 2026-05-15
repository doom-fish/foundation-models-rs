//! API-surface coverage harness for `foundation-models`.
//!
//! Parses Apple's textual `.swiftinterface` for `FoundationModels.framework`
//! and verifies that every public symbol on the types we wrap
//! (`SystemLanguageModel`, `LanguageModelSession`, `GenerationOptions`,
//! `GenerationOptions.SamplingMode`) is either:
//!
//! * referenced by name in our Swift bridge
//!   (`swift-bridge/Sources/FoundationModelsBridge/FoundationModels.swift`), or
//! * listed in the per-type `intentionally_omitted()` allowlist with a reason.
//!
//! Coverage is measured per-type rather than across the whole framework
//! because v0.1 only wraps the chat surface — `Tool`, `Generable`,
//! `GenerationGuide`, and friends are explicitly out of scope for this minor
//! release and live in their own omitted sets.

#![allow(clippy::cast_precision_loss, clippy::iter_on_single_items)]

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

fn sdk_root() -> PathBuf {
    let out = Command::new("xcrun")
        .args(["--sdk", "macosx", "--show-sdk-path"])
        .output()
        .expect("xcrun must be available");
    assert!(out.status.success());
    PathBuf::from(String::from_utf8(out.stdout).unwrap().trim().to_string())
}

fn read_swiftinterface() -> String {
    let sdk = sdk_root();
    let path = sdk.join(
        "System/Library/Frameworks/FoundationModels.framework/\
         Modules/FoundationModels.swiftmodule/arm64e-apple-macos.swiftinterface",
    );
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("can't read {}: {e}", path.display()))
}

fn read_our_bridge() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("swift-bridge/Sources/FoundationModelsBridge/FoundationModels.swift");
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("can't read {}: {e}", path.display()))
}

/// Extract the public surface of a single type by isolating the lines
/// between the opening `(class|struct|enum) NAME` and the matching `}`.
///
/// Returns the bare member names (without argument labels): `respond`,
/// `streamResponse`, `availability`, etc.
fn extract_type_surface(swiftinterface: &str, type_name: &str) -> BTreeSet<String> {
    // Locate the type declaration line.
    let needle =
        regex_lite::Regex::new(&format!(r"\b(class|struct|enum)\s+{type_name}\b"))
            .unwrap();
    let start_match = needle
        .find(swiftinterface)
        .unwrap_or_else(|| panic!("can't locate `{type_name}` in swiftinterface"));
    let body_start = start_match.end();

    // Naive brace matching — works for the canonical Apple swiftinterface
    // formatting where one declaration spans multiple lines but braces are
    // balanced.
    let bytes = swiftinterface.as_bytes();
    let mut depth: i32 = 0;
    let mut found_open = false;
    let mut end = body_start;
    for (i, &b) in bytes.iter().enumerate().skip(body_start) {
        match b {
            b'{' => {
                depth += 1;
                found_open = true;
            }
            b'}' => {
                depth -= 1;
                if found_open && depth == 0 {
                    end = i;
                    break;
                }
            }
            _ => {}
        }
    }
    let body = &swiftinterface[body_start..end];

    let mut surface = BTreeSet::new();
    // public init(...)
    let init_re = regex_lite::Regex::new(r"\bpublic\s+(?:convenience\s+)?init\b").unwrap();
    if init_re.is_match(body) {
        surface.insert("init".to_string());
    }
    // Functions: `public func NAME(`, `final public func NAME(`, etc.
    let func_re =
        regex_lite::Regex::new(r"\bpublic\s+(?:[a-zA-Z@_][\w@()<>=, ]*\s+)?func\s+([a-zA-Z_][A-Za-z0-9_]*)").unwrap();
    for c in func_re.captures_iter(body) {
        surface.insert(c[1].to_string());
    }
    // Vars: `public var NAME` / `public let NAME` / `public static var NAME`
    let var_re =
        regex_lite::Regex::new(r"\bpublic\s+(?:static\s+|class\s+|final\s+)*(?:var|let)\s+([a-zA-Z_][A-Za-z0-9_]*)").unwrap();
    for c in var_re.captures_iter(body) {
        surface.insert(c[1].to_string());
    }
    // Enum cases: `case foo`
    let case_re = regex_lite::Regex::new(r"\bcase\s+([a-zA-Z_][A-Za-z0-9_]*)").unwrap();
    for c in case_re.captures_iter(body) {
        surface.insert(c[1].to_string());
    }
    // Static factories `public static func` already caught above.

    surface
}

#[derive(Default)]
struct Report {
    type_name: &'static str,
    apple: BTreeSet<String>,
    referenced: BTreeSet<String>,
    omitted: BTreeSet<String>,
}

impl Report {
    fn run(self) -> Result<(), String> {
        let wrapped: BTreeSet<&String> = self.apple.intersection(&self.referenced).collect();
        let missing: BTreeSet<&String> = self
            .apple
            .difference(&self.referenced)
            .filter(|s| !self.omitted.contains(*s))
            .collect();

        let coverable = wrapped.len() + missing.len();
        let pct = if coverable == 0 {
            100.0
        } else {
            wrapped.len() as f64 / coverable as f64 * 100.0
        };

        println!(
            "\n=== {} API coverage ===\n\
             Apple symbols:     {}\n\
             Intentionally omitted: {}\n\
             ----\n\
             Coverable: {coverable}\n\
             Wrapped:   {} ({pct:.1}%)\n\
             Missing:   {}",
            self.type_name,
            self.apple.len(),
            self.omitted.len(),
            wrapped.len(),
            missing.len(),
        );
        if !missing.is_empty() {
            println!("\n--- Missing ---");
            for s in &missing {
                println!("  - {s}");
            }
        }
        if pct < 100.0 {
            return Err(format!("{} coverable coverage is {pct:.1}%", self.type_name));
        }
        Ok(())
    }
}

fn references_in_bridge(symbols: &BTreeSet<String>) -> BTreeSet<String> {
    let bridge = read_our_bridge();
    let aliases = swift_aliases();
    symbols
        .iter()
        .filter(|name| {
            let needle = format!(r"\b{}", regex_lite::escape(name));
            if regex_lite::Regex::new(&needle).unwrap().is_match(&bridge) {
                return true;
            }
            // Alias check: e.g. `init` is referenced via `LanguageModelSession(...)`.
            if let Some(alias_form) = aliases.get(name.as_str()) {
                return bridge.contains(alias_form);
            }
            false
        })
        .cloned()
        .collect()
}

/// Swift constructors are imported as `init`; we call them as `TypeName(...)`.
/// Map the Swift-interface name onto the textual form our bridge actually uses.
fn swift_aliases() -> std::collections::BTreeMap<&'static str, &'static str> {
    [
        ("init", "Session("),    // LanguageModelSession( + GenerationOptions(
    ]
    .into_iter()
    .collect()
}

// ---- Test cases ----

#[test]
fn system_language_model_coverage() {
    let si = read_swiftinterface();
    let apple = extract_type_surface(&si, "SystemLanguageModel");
    let referenced = references_in_bridge(&apple);
    let omitted: BTreeSet<String> = [
        // We wrap `default` + `availability`. The rest are advanced features
        // for v0.2:
        "supportedLanguages", // language list discovery — v0.2
        "languageRecognizer", // tool-call language hint — v0.2
        "guardrails",         // user-overridable guardrails — v0.2
        "use",                // useCase parameterisation — v0.2
        "isAvailable",        // we use `availability` directly
        "init",               // we don't construct, we read .default
        "useCase",            // v0.2
        "Availability",       // nested type — surface accessed via .available pattern match
        "Reason",             // nested type — surface accessed via .deviceNotEligible etc.
        "Guardrails",         // nested type — v0.2
        "UseCase",            // nested type — v0.2
        "SupportedLanguages", // nested type — v0.2
        // Apple-defined alternative model handles. We only ship `default`.
        "contentTagging",
        "general",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    Report {
        type_name: "SystemLanguageModel",
        apple,
        referenced,
        omitted,
    }
    .run()
    .unwrap();
}

#[test]
fn language_model_session_coverage() {
    let si = read_swiftinterface();
    let apple = extract_type_surface(&si, "LanguageModelSession");
    let referenced = references_in_bridge(&apple);
    let omitted: BTreeSet<String> = [
        // Out of scope for v0.1 — string-only chat is the v0.1 surface.
        "transcript",          // transcript inspection — v0.2
        "isResponding",        // KVO-style polling — v0.2
        "prewarm",             // pre-warm the model — v0.2
        "Response",            // nested type — wrapper just gets `.content`
        "logFeedbackAttachment", // user-feedback feature — v0.2
        "GenerationError",     // nested error type — wrapper maps via switch
        "Error",               // alias for GenerationError
        "ToolCallError",       // nested tool-call error — v0.2 (no Tool support)
        "Refusal",             // nested type for refusal explanations — v0.2
        "PromptBuilder",       // result builder for fluent prompts — v0.2
        "InstructionsBuilder", // result builder for instructions — v0.2
        // Schema-driven structured generation — v0.2 (Generable macro support):
        "schema",
        "type",
        "includeSchemaInPrompt",
        "generating",
        // Stream variants beyond plain string — schema/Generable variants v0.2:
        "ResponseStream",
        "Snapshot",
        // Nested Response<Content> fields — we expose only `.content` (the
        // primary text answer); the metadata fields below are v0.2.
        "rawContent",
        "transcriptEntries",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    Report {
        type_name: "LanguageModelSession",
        apple,
        referenced,
        omitted,
    }
    .run()
    .unwrap();
}

#[test]
fn generation_options_coverage() {
    let si = read_swiftinterface();
    let apple = extract_type_surface(&si, "GenerationOptions");
    let referenced = references_in_bridge(&apple);
    Report {
        type_name: "GenerationOptions",
        apple,
        referenced,
        omitted: BTreeSet::new(), // every public field is wrapped
    }
    .run()
    .unwrap();
}

#[test]
fn sampling_mode_coverage() {
    let si = read_swiftinterface();
    // GenerationOptions.SamplingMode is a nested type. Search for it in the
    // GenerationOptions body.
    let apple = extract_type_surface(&si, "SamplingMode");
    let referenced = references_in_bridge(&apple);
    let omitted: BTreeSet<String> = [
        // Hash/Equatable/etc. compiler-synth noise that the regex catches.
        "hash",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    Report {
        type_name: "GenerationOptions.SamplingMode",
        apple,
        referenced,
        omitted,
    }
    .run()
    .unwrap();
}

#[test]
fn response_stream_snapshot_coverage() {
    // `LanguageModelSession.ResponseStream<Content>.Snapshot` is the per-token
    // delta type yielded by `streamResponse(to:options:)`. Our streaming
    // bridge reads `partial.content` to get the accumulated text — verify
    // the field is actually called `content` in Apple's swiftinterface so a
    // future SDK rename can't silently break us.
    let si = read_swiftinterface();
    let apple = extract_type_surface(&si, "Snapshot");
    let referenced = references_in_bridge(&apple);
    let omitted: BTreeSet<String> = [
        // We surface only `.content` — the accumulated PartiallyGenerated value.
        // `rawContent` exposes the underlying `GeneratedContent` for schema-
        // driven structured generation, which lands in v0.2.
        "rawContent",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    Report {
        type_name: "LanguageModelSession.ResponseStream.Snapshot",
        apple,
        referenced,
        omitted,
    }
    .run()
    .unwrap();
}
