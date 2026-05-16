// FoundationModels Bridge - SystemLanguageModel & LanguageModelSession
//
// Bridges Swift's `async throws` FoundationModels APIs into a C-callable
// surface for Rust. All async work runs in detached Tasks; the Rust caller
// passes a context pointer + C function pointer that the Task invokes on
// completion (or per streamed chunk).
//
// Pointers crossing the boundary:
// * Session pointers come from `Unmanaged.passRetained(...).toOpaque()` and
//   must be released via `fm_object_release`.
// * Context pointers are opaque to Swift; the Rust side casts them back.
// * String returns are heap-allocated via `strdup`; Rust frees them with
//   `fm_string_free`.

import Foundation

#if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
import FoundationModels
#endif

// MARK: - Status Codes
//
// Mirrored 1:1 in src/ffi/mod.rs::status. Plain Int32 module constants
// keep the @_cdecl call sites self-contained.

private let FM_OK: Int32 = 0
private let FM_INVALID_ARGUMENT: Int32 = -1
private let FM_MODEL_UNAVAILABLE: Int32 = -2
private let FM_CANCELLED: Int32 = -3
private let FM_GUARDRAIL_VIOLATION: Int32 = -4
private let FM_CONTEXT_WINDOW_EXCEEDED: Int32 = -5
private let FM_UNSUPPORTED_LANGUAGE: Int32 = -6
private let FM_ASSETS_UNAVAILABLE: Int32 = -7
private let FM_RATE_LIMITED: Int32 = -8
private let FM_DECODING_FAILURE: Int32 = -9
private let FM_REFUSAL: Int32 = -10
private let FM_CONCURRENT_REQUESTS: Int32 = -11
private let FM_UNSUPPORTED_GUIDE: Int32 = -12
private let FM_UNKNOWN: Int32 = -99

// MARK: - String Helpers

@_cdecl("fm_string_dup")
public func fm_string_dup(_ str: UnsafePointer<CChar>?) -> UnsafeMutablePointer<CChar>? {
    guard let str = str else { return nil }
    return strdup(str)
}

@_cdecl("fm_string_free")
public func fm_string_free(_ str: UnsafeMutablePointer<CChar>?) {
    guard let str = str else { return }
    free(str)
}

private func ffiString(_ s: String) -> UnsafeMutablePointer<CChar>? {
    return s.withCString { strdup($0) }
}

// MARK: - Object Lifetime

@_cdecl("fm_object_release")
public func fm_object_release(_ ptr: UnsafeMutableRawPointer?) {
    guard let ptr = ptr else { return }
    Unmanaged<AnyObject>.fromOpaque(ptr).release()
}

// MARK: - Availability

@_cdecl("fm_system_model_is_available")
public func fm_system_model_is_available() -> Bool {
    #if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
        let model = SystemLanguageModel.default
        if case .available = model.availability {
            return true
        }
        return false
    } else {
        return false
    }
    #else
    return false
    #endif
}

/// Returns 0 = available, 1 = device not eligible, 2 = AI not enabled,
/// 3 = model not ready, 4 = unknown unavailable, -1 = OS too old.
@_cdecl("fm_system_model_availability_code")
public func fm_system_model_availability_code() -> Int32 {
    #if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
        let model = SystemLanguageModel.default
        switch model.availability {
        case .available:
            return 0
        case .unavailable(let reason):
            switch reason {
            case .deviceNotEligible: return 1
            case .appleIntelligenceNotEnabled: return 2
            case .modelNotReady: return 3
            @unknown default: return 4
            }
        @unknown default:
            return 4
        }
    } else {
        return -1
    }
    #else
    return -1
    #endif
}

// MARK: - Session Lifecycle

/// Create a session. `instructions` may be NULL for the default system prompt.
/// Returns an opaque retained pointer; release with `fm_object_release`.
/// On macOS < 26 returns NULL.
@_cdecl("fm_session_create")
public func fm_session_create(_ instructions: UnsafePointer<CChar>?) -> UnsafeMutableRawPointer? {
    #if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
        let session: LanguageModelSession
        if let instructions = instructions {
            let str = String(cString: instructions)
            session = LanguageModelSession(instructions: Instructions(str))
        } else {
            session = LanguageModelSession()
        }
        return Unmanaged.passRetained(session).toOpaque()
    } else {
        return nil
    }
    #else
    return nil
    #endif
}

/// Pre-warm the model so the next call is faster. Apple loads the model
/// weights + initialises the inference engine. Optionally accepts a
/// short hint prompt to bias the cache. Returns immediately.
@_cdecl("fm_session_prewarm")
public func fm_session_prewarm(_ sessionPtr: UnsafeMutableRawPointer) {
    #if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
        let session = Unmanaged<LanguageModelSession>.fromOpaque(sessionPtr).takeUnretainedValue()
        session.prewarm()
    }
    #endif
}

/// Query whether the session is currently producing a response (i.e. a
/// previous `respond` or `streamResponse` is still in flight).
@_cdecl("fm_session_is_responding")
public func fm_session_is_responding(_ sessionPtr: UnsafeMutableRawPointer) -> Bool {
    #if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
        let session = Unmanaged<LanguageModelSession>.fromOpaque(sessionPtr).takeUnretainedValue()
        return session.isResponding
    }
    #endif
    return false
}

// MARK: - Generation Options
//
// Options are passed as individual scalar parameters rather than a struct
// pointer because @_cdecl cannot represent a Swift struct in Objective-C.
// `temperature == NaN` means "leave default"; `maximumResponseTokens == 0`
// means "no explicit limit".
// `samplingMode`: 0 = default, 1 = greedy, 2 = top-k, 3 = top-p (nucleus).

#if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
@available(macOS 26.0, *)
private func buildOptions(
    temperature: Double,
    maxTokens: Int32,
    samplingMode: Int32,
    topK: Int32,
    topP: Double
) -> GenerationOptions {
    var sampling: GenerationOptions.SamplingMode? = nil
    switch samplingMode {
    case 1: sampling = .greedy
    case 2 where topK > 0: sampling = .random(top: Int(topK))
    case 3 where topP > 0: sampling = .random(probabilityThreshold: topP)
    default: sampling = nil
    }
    let temp: Double? = temperature.isNaN ? nil : temperature
    let maxT: Int? = maxTokens > 0 ? Int(maxTokens) : nil
    return GenerationOptions(
        sampling: sampling,
        temperature: temp,
        maximumResponseTokens: maxT
    )
}
#endif

// MARK: - Respond (single-shot)
//
// NB: @_cdecl can't accept Swift typealiases for C function pointers, so the
// callback signature is inlined verbatim in the parameter list.

@_cdecl("fm_session_respond")
public func fm_session_respond(
    _ sessionPtr: UnsafeMutableRawPointer,
    _ prompt: UnsafePointer<CChar>,
    _ temperature: Double,
    _ maxTokens: Int32,
    _ samplingMode: Int32,
    _ topK: Int32,
    _ topP: Double,
    _ context: UnsafeMutableRawPointer?,
    _ callback: @convention(c) (
        UnsafeMutableRawPointer?,
        UnsafeMutablePointer<CChar>?,
        UnsafeMutablePointer<CChar>?,
        Int32
    ) -> Void
) {
    #if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
        let session = Unmanaged<LanguageModelSession>.fromOpaque(sessionPtr).takeUnretainedValue()
        let promptStr = String(cString: prompt)
        let opts = buildOptions(
            temperature: temperature,
            maxTokens: maxTokens,
            samplingMode: samplingMode,
            topK: topK,
            topP: topP
        )
        Task.detached {
            do {
                let response = try await session.respond(
                    to: Prompt(promptStr),
                    options: opts
                )
                let cstr = ffiString(response.content)
                callback(context, cstr, nil, FM_OK)
            } catch {
                let (code, message) = mapError(error)
                let cstr = ffiString(message)
                callback(context, nil, cstr, code)
            }
        }
        return
    }
    #endif
    let cstr = ffiString("FoundationModels requires macOS 26.0 or newer")
    callback(context, nil, cstr, FM_MODEL_UNAVAILABLE)
}

// MARK: - Stream Response (chunked)

@_cdecl("fm_session_stream_response")
public func fm_session_stream_response(
    _ sessionPtr: UnsafeMutableRawPointer,
    _ prompt: UnsafePointer<CChar>,
    _ temperature: Double,
    _ maxTokens: Int32,
    _ samplingMode: Int32,
    _ topK: Int32,
    _ topP: Double,
    _ context: UnsafeMutableRawPointer?,
    _ callback: @convention(c) (
        UnsafeMutableRawPointer?,
        UnsafeMutablePointer<CChar>?,
        Bool,
        Int32
    ) -> Void
) {
    #if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
        let session = Unmanaged<LanguageModelSession>.fromOpaque(sessionPtr).takeUnretainedValue()
        let promptStr = String(cString: prompt)
        let opts = buildOptions(
            temperature: temperature,
            maxTokens: maxTokens,
            samplingMode: samplingMode,
            topK: topK,
            topP: topP
        )
        Task.detached {
            do {
                let stream = session.streamResponse(
                    to: Prompt(promptStr),
                    options: opts
                )
                var lastEmitted = ""
                for try await partial in stream {
                    // partial is a Snapshot whose `content` is the
                    // accumulated PartiallyGenerated value (a plain String
                    // for the string-typed streamResponse overload).
                    // Emit only the delta so Rust callers can print
                    // without de-duplicating.
                    let full = partial.content
                    let delta: String
                    if full.hasPrefix(lastEmitted) {
                        delta = String(full.dropFirst(lastEmitted.count))
                    } else {
                        delta = full
                    }
                    lastEmitted = full
                    if !delta.isEmpty {
                        let cstr = ffiString(delta)
                        callback(context, cstr, false, FM_OK)
                    }
                }
                callback(context, nil, true, FM_OK)
            } catch {
                let (code, message) = mapError(error)
                let cstr = ffiString(message)
                callback(context, cstr, true, code)
            }
        }
        return
    }
    #endif
    let cstr = ffiString("FoundationModels requires macOS 26.0 or newer")
    callback(context, cstr, true, FM_MODEL_UNAVAILABLE)
}

// MARK: - Error Mapping

#if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
@available(macOS 26.0, *)
private func mapError(_ error: Error) -> (Int32, String) {
    if let lmError = error as? LanguageModelSession.GenerationError {
        let msg = lmError.localizedDescription
        switch lmError {
        case .guardrailViolation:        return (FM_GUARDRAIL_VIOLATION, msg)
        case .exceededContextWindowSize: return (FM_CONTEXT_WINDOW_EXCEEDED, msg)
        case .unsupportedLanguageOrLocale: return (FM_UNSUPPORTED_LANGUAGE, msg)
        case .assetsUnavailable:         return (FM_ASSETS_UNAVAILABLE, msg)
        case .rateLimited:               return (FM_RATE_LIMITED, msg)
        case .decodingFailure:           return (FM_DECODING_FAILURE, msg)
        case .refusal:                   return (FM_REFUSAL, msg)
        case .concurrentRequests:        return (FM_CONCURRENT_REQUESTS, msg)
        case .unsupportedGuide:          return (FM_UNSUPPORTED_GUIDE, msg)
        @unknown default:                return (FM_UNKNOWN, msg)
        }
    }
    let nsError = error as NSError
    if nsError.code == NSUserCancelledError {
        return (FM_CANCELLED, error.localizedDescription)
    }
    return (FM_UNKNOWN, error.localizedDescription)
}
#else
private func mapError(_ error: Error) -> (Int32, String) {
    return (FM_UNKNOWN, error.localizedDescription)
}
#endif

// MARK: - Schema-driven respond (v0.4)
//
// Takes a JSON-schema-shaped Rust string, builds a
// `DynamicGenerationSchema`, runs respond(schema:prompt:), and returns
// the model's `GeneratedContent.jsonString`.
//
// Supported schema shape (a strict subset of JSON Schema):
// {
//   "type": "object",
//   "name": "Root",            // optional, defaults to "Root"
//   "description": "…",        // optional
//   "properties": {
//      "title": { "type": "string", "description": "…", "optional": false },
//      "year":  { "type": "integer" },
//      "tags":  { "type": "array", "items": { "type": "string" }, "min": 1, "max": 5 }
//   }
// }
//
// Primitive type strings: "string", "integer" (Int), "number" (Double),
// "boolean", "array", "object".

#if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK

@available(macOS 26.0, *)
private func buildDynamicSchema(from json: Any, name: String) throws -> DynamicGenerationSchema {
    guard let dict = json as? [String: Any] else {
        throw NSError(domain: "fm-bridge", code: -1, userInfo: [
            NSLocalizedDescriptionKey: "schema must be a JSON object"
        ])
    }
    let typeStr = (dict["type"] as? String) ?? "object"
    let desc = dict["description"] as? String

    switch typeStr {
    case "object":
        let nm = (dict["name"] as? String) ?? name
        var props: [DynamicGenerationSchema.Property] = []
        if let propDict = dict["properties"] as? [String: Any] {
            for (pname, pval) in propDict {
                let psub = try buildDynamicSchema(from: pval, name: pname)
                let pdesc = (pval as? [String: Any])?["description"] as? String
                let opt = ((pval as? [String: Any])?["optional"] as? Bool) ?? false
                props.append(DynamicGenerationSchema.Property(
                    name: pname, description: pdesc, schema: psub, isOptional: opt))
            }
        }
        return DynamicGenerationSchema(name: nm, description: desc, properties: props)
    case "array":
        let itemJson = dict["items"] ?? ["type": "string"]
        let itemSchema = try buildDynamicSchema(from: itemJson, name: "Item")
        let minE = (dict["min"] as? Int)
        let maxE = (dict["max"] as? Int)
        return DynamicGenerationSchema(arrayOf: itemSchema,
                                       minimumElements: minE,
                                       maximumElements: maxE)
    case "string":
        return DynamicGenerationSchema(type: String.self, guides: [])
    case "integer":
        return DynamicGenerationSchema(type: Int.self, guides: [])
    case "number":
        return DynamicGenerationSchema(type: Double.self, guides: [])
    case "boolean":
        return DynamicGenerationSchema(type: Bool.self, guides: [])
    default:
        throw NSError(domain: "fm-bridge", code: -2, userInfo: [
            NSLocalizedDescriptionKey: "unsupported schema type: \(typeStr)"
        ])
    }
}

#endif

@_cdecl("fm_session_respond_with_schema")
public func fm_session_respond_with_schema(
    _ sessionPtr: UnsafeMutableRawPointer,
    _ prompt: UnsafePointer<CChar>,
    _ schemaJson: UnsafePointer<CChar>,
    _ includeSchemaInPrompt: Bool,
    _ temperature: Double,
    _ maxTokens: Int32,
    _ samplingMode: Int32,
    _ topK: Int32,
    _ topP: Double,
    _ context: UnsafeMutableRawPointer?,
    _ callback: @convention(c) (
        UnsafeMutableRawPointer?,
        UnsafeMutablePointer<CChar>?,
        UnsafeMutablePointer<CChar>?,
        Int32
    ) -> Void
) {
    #if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
        let session = Unmanaged<LanguageModelSession>.fromOpaque(sessionPtr).takeUnretainedValue()
        let promptStr = String(cString: prompt)
        let schemaStr = String(cString: schemaJson)
        let opts = buildOptions(
            temperature: temperature,
            maxTokens: maxTokens,
            samplingMode: samplingMode,
            topK: topK,
            topP: topP
        )

        guard let schemaData = schemaStr.data(using: .utf8),
              let schemaParsed = try? JSONSerialization.jsonObject(with: schemaData, options: []) else {
            let cstr = ffiString("schema JSON is not valid")
            callback(context, nil, cstr, FM_UNKNOWN)
            return
        }
        do {
            let dyn = try buildDynamicSchema(from: schemaParsed, name: "Root")
            let schema = try GenerationSchema(root: dyn, dependencies: [])
            Task.detached {
                do {
                    let response = try await session.respond(
                        to: Prompt(promptStr),
                        schema: schema,
                        includeSchemaInPrompt: includeSchemaInPrompt,
                        options: opts
                    )
                    let cstr = ffiString(response.content.jsonString)
                    callback(context, cstr, nil, FM_OK)
                } catch {
                    let (code, message) = mapError(error)
                    let cstr = ffiString(message)
                    callback(context, nil, cstr, code)
                }
            }
            return
        } catch {
            let cstr = ffiString("schema build failed: \(error.localizedDescription)")
            callback(context, nil, cstr, FM_UNKNOWN)
            return
        }
    }
    #endif
    let cstr = ffiString("FoundationModels requires macOS 26.0 or newer")
    callback(context, nil, cstr, FM_MODEL_UNAVAILABLE)
}

// MARK: - Transcript export (v0.5)

@_cdecl("fm_session_transcript_json")
public func fm_session_transcript_json(
    _ sessionPtr: UnsafeMutableRawPointer
) -> UnsafeMutablePointer<CChar>? {
    #if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
        let session = Unmanaged<LanguageModelSession>.fromOpaque(sessionPtr).takeUnretainedValue()
        // Best-effort transcript -> JSON via JSONEncoder.
        let transcript = session.transcript
        let encoder = JSONEncoder()
        if let data = try? encoder.encode(transcript),
           let s = String(data: data, encoding: .utf8) {
            return ffiString(s)
        }
        return ffiString("{}")
    }
    #endif
    return ffiString("{}")
}

@_cdecl("fm_session_log_feedback")
public func fm_session_log_feedback(
    _ sessionPtr: UnsafeMutableRawPointer,
    _ sentiment: Int32,
    _ description: UnsafePointer<CChar>?
) {
    #if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
        let session = Unmanaged<LanguageModelSession>.fromOpaque(sessionPtr).takeUnretainedValue()
        let s: LanguageModelFeedbackAttachment.Sentiment
        switch sentiment {
        case 1: s = .positive
        case -1: s = .negative
        default: s = .neutral
        }
        var issues: [LanguageModelFeedbackAttachment.Issue] = []
        if let p = description {
            let str = String(cString: p)
            issues.append(.init(category: .unhelpful, explanation: str))
        }
        session.logFeedbackAttachment(sentiment: s, issues: issues, desiredOutput: nil)
    }
    #endif
}
