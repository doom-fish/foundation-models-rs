import Foundation

#if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
import FoundationModels
#endif

#if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
@available(macOS 26.0, *)
func writeErrorOut(
    _ errorOut: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?,
    _ payload: BridgeErrorPayload
) {
    errorOut?.pointee = ffiString(encodeErrorPayload(payload))
}

@available(macOS 26.0, *)
func generationErrorPayload(_ error: LanguageModelSession.GenerationError) -> BridgeErrorPayload {
    let message = error.localizedDescription
    let recoverySuggestion = error.recoverySuggestion
    let failureReason = error.failureReason

    switch error {
    case .guardrailViolation(let context),
         .exceededContextWindowSize(let context),
         .unsupportedLanguageOrLocale(let context),
         .assetsUnavailable(let context),
         .rateLimited(let context),
         .decodingFailure(let context),
         .concurrentRequests(let context),
         .unsupportedGuide(let context):
        return BridgeErrorPayload(
            message: message,
            recoverySuggestion: recoverySuggestion,
            failureReason: failureReason,
            generationErrorContext: BridgeErrorContext(debugDescription: context.debugDescription),
            refusal: nil,
            toolCallError: nil,
            schemaErrorContext: nil
        )
    case .refusal(let refusal, let context):
        return BridgeErrorPayload(
            message: message,
            recoverySuggestion: recoverySuggestion,
            failureReason: failureReason,
            generationErrorContext: BridgeErrorContext(debugDescription: context.debugDescription),
            refusal: bridgeRefusal(refusal),
            toolCallError: nil,
            schemaErrorContext: nil
        )
    @unknown default:
        return BridgeErrorPayload(
            message: message,
            recoverySuggestion: recoverySuggestion,
            failureReason: failureReason,
            generationErrorContext: nil,
            refusal: nil,
            toolCallError: nil,
            schemaErrorContext: nil
        )
    }
}

@available(macOS 26.0, *)
func schemaErrorPayload(_ error: GenerationSchema.SchemaError) -> BridgeErrorPayload {
    let context: GenerationSchema.SchemaError.Context
    switch error {
    case .duplicateType(_, _, let errorContext),
         .duplicateProperty(_, _, let errorContext),
         .emptyTypeChoices(_, let errorContext),
         .undefinedReferences(_, _, let errorContext):
        context = errorContext
    @unknown default:
        return BridgeErrorPayload(
            message: error.localizedDescription,
            recoverySuggestion: error.recoverySuggestion,
            failureReason: nil,
            generationErrorContext: nil,
            refusal: nil,
            toolCallError: nil,
            schemaErrorContext: nil
        )
    }

    return BridgeErrorPayload(
        message: error.localizedDescription,
        recoverySuggestion: error.recoverySuggestion,
        failureReason: nil,
        generationErrorContext: nil,
        refusal: nil,
        toolCallError: nil,
        schemaErrorContext: BridgeErrorContext(debugDescription: context.debugDescription)
    )
}

@available(macOS 26.0, *)
func toolCallErrorPayload(_ error: LanguageModelSession.ToolCallError) -> BridgeErrorPayload {
    BridgeErrorPayload(
        message: error.localizedDescription,
        recoverySuggestion: nil,
        failureReason: nil,
        generationErrorContext: nil,
        refusal: nil,
        toolCallError: BridgeToolCallErrorPayload(
            tool: bridgeToolDefinition(from: error.tool),
            underlyingError: error.underlyingError.localizedDescription
        ),
        schemaErrorContext: nil
    )
}

@available(macOS 26.0, *)
func encodedTextResponse(_ response: LanguageModelSession.Response<String>) throws -> String {
    let transcriptJSON = try encodeTranscriptJSON(entries: response.transcriptEntries)
    let payload = BridgeTextResponse(
        content: response.content,
        rawContent: bridgeGeneratedContent(response.rawContent),
        transcriptJSON: transcriptJSON
    )
    return try encodeBridge(payload)
}

@available(macOS 26.0, *)
func streamTextResponse(
    _ stream: LanguageModelSession.ResponseStream<String>,
    context: UnsafeMutableRawPointer?,
    callback: @convention(c) (
        UnsafeMutableRawPointer?,
        UnsafeMutablePointer<CChar>?,
        Bool,
        Int32
    ) -> Void
) async {
    do {
        var lastEmitted = ""
        for try await snapshot in stream {
            let full = snapshot.content
            let delta: String
            if full.hasPrefix(lastEmitted) {
                delta = String(full.dropFirst(lastEmitted.count))
            } else {
                delta = full
            }
            lastEmitted = full
            let payload = BridgeTextStreamSnapshot(
                delta: delta,
                content: full,
                rawContent: bridgeGeneratedContent(snapshot.rawContent)
            )
            callback(context, ffiString(try encodeBridge(payload)), false, FM_OK)
        }
        callback(context, nil, true, FM_OK)
    } catch {
        let (code, message) = mapError(error)
        callback(context, ffiString(message), true, code)
    }
}
#endif

@_cdecl("fm_generation_id_create")
public func fm_generation_id_create(
    _ outputOut: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?,
    _ errorOut: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    #if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
        let handle = GenerationIDRegistry.shared.register(GenerationID())
        outputOut?.pointee = ffiString((try? encodeBridge(handle)) ?? "")
        return FM_OK
    }
    #endif
    writeErrorOut(errorOut, "FoundationModels requires macOS 26.0 or newer")
    return FM_MODEL_UNAVAILABLE
}

@_cdecl("fm_decimal_to_generated_content_json")
public func fm_decimal_to_generated_content_json(
    _ decimalString: UnsafePointer<CChar>,
    _ outputOut: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?,
    _ errorOut: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    #if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
        guard let decimal = Decimal(string: String(cString: decimalString)) else {
            writeErrorOut(errorOut, BridgeErrorPayload(
                message: "decimal value is invalid",
                recoverySuggestion: nil,
                failureReason: nil,
                generationErrorContext: nil,
                refusal: nil,
                toolCallError: nil,
                schemaErrorContext: nil
            ))
            return FM_INVALID_ARGUMENT
        }
        outputOut?.pointee = ffiString(decimal.generatedContent.jsonString)
        return FM_OK
    }
    #endif
    writeErrorOut(errorOut, "FoundationModels requires macOS 26.0 or newer")
    return FM_MODEL_UNAVAILABLE
}

@_cdecl("fm_decimal_from_generated_content_json")
public func fm_decimal_from_generated_content_json(
    _ generatedContentJSON: UnsafePointer<CChar>,
    _ outputOut: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?,
    _ errorOut: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    #if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
        do {
            let content = try GeneratedContent(json: String(cString: generatedContentJSON))
            let decimal = try Decimal(content)
            outputOut?.pointee = ffiString(NSDecimalNumber(decimal: decimal).stringValue)
            return FM_OK
        } catch {
            let (code, message) = mapError(error)
            errorOut?.pointee = ffiString(message)
            return code
        }
    }
    #endif
    writeErrorOut(errorOut, "FoundationModels requires macOS 26.0 or newer")
    return FM_MODEL_UNAVAILABLE
}

@_cdecl("fm_refusal_explanation_json")
public func fm_refusal_explanation_json(
    _ refusalToken: UnsafePointer<CChar>,
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
        let bridgeRefusal = BridgeRefusal(token: String(cString: refusalToken))
        guard let refusal = RefusalRegistry.shared.resolve(bridgeRefusal) else {
            callback(context, nil, ffiString("unknown refusal token"), FM_INVALID_ARGUMENT)
            return
        }
        Task.detached {
            do {
                let response = try await refusal.explanation
                callback(context, ffiString(try encodedTextResponse(response)), nil, FM_OK)
            } catch {
                let (code, message) = mapError(error)
                callback(context, nil, ffiString(message), code)
            }
        }
        return
    }
    #endif
    callback(context, nil, ffiString("FoundationModels requires macOS 26.0 or newer"), FM_MODEL_UNAVAILABLE)
}

@_cdecl("fm_refusal_explanation_from_transcript_json")
public func fm_refusal_explanation_from_transcript_json(
    _ transcriptJSON: UnsafePointer<CChar>,
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
        Task.detached {
            do {
                let transcript = try decodeTranscript(from: String(cString: transcriptJSON))
                let refusal = LanguageModelSession.GenerationError.Refusal(
                    transcriptEntries: Array(transcript)
                )
                let response = try await refusal.explanation
                callback(context, ffiString(try encodedTextResponse(response)), nil, FM_OK)
            } catch {
                let (code, message) = mapError(error)
                callback(context, nil, ffiString(message), code)
            }
        }
        return
    }
    #endif
    callback(context, nil, ffiString("FoundationModels requires macOS 26.0 or newer"), FM_MODEL_UNAVAILABLE)
}

@_cdecl("fm_refusal_explanation_stream")
public func fm_refusal_explanation_stream(
    _ refusalToken: UnsafePointer<CChar>,
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
        let bridgeRefusal = BridgeRefusal(token: String(cString: refusalToken))
        guard let refusal = RefusalRegistry.shared.resolve(bridgeRefusal) else {
            callback(context, ffiString("unknown refusal token"), true, FM_INVALID_ARGUMENT)
            return
        }
        Task.detached {
            await streamTextResponse(refusal.explanationStream, context: context, callback: callback)
        }
        return
    }
    #endif
    callback(context, ffiString("FoundationModels requires macOS 26.0 or newer"), true, FM_MODEL_UNAVAILABLE)
}

@_cdecl("fm_refusal_explanation_stream_from_transcript_json")
public func fm_refusal_explanation_stream_from_transcript_json(
    _ transcriptJSON: UnsafePointer<CChar>,
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
        Task.detached {
            do {
                let transcript = try decodeTranscript(from: String(cString: transcriptJSON))
                let refusal = LanguageModelSession.GenerationError.Refusal(
                    transcriptEntries: Array(transcript)
                )
                await streamTextResponse(refusal.explanationStream, context: context, callback: callback)
            } catch {
                let (code, message) = mapError(error)
                callback(context, ffiString(message), true, code)
            }
        }
        return
    }
    #endif
    callback(context, ffiString("FoundationModels requires macOS 26.0 or newer"), true, FM_MODEL_UNAVAILABLE)
}
