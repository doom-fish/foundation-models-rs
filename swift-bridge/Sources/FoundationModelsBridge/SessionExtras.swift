import Foundation

#if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
import FoundationModels
#endif

#if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
@available(macOS 26.0, *)
func feedbackSentiment(from rawValue: String?) -> LanguageModelFeedback.Sentiment? {
    switch rawValue {
    case nil:
        return nil
    case "positive":
        return .positive
    case "negative":
        return .negative
    default:
        return .neutral
    }
}

@available(macOS 26.0, *)
func feedbackIssueCategory(from rawValue: String) -> LanguageModelFeedback.Issue.Category {
    switch rawValue {
    case "too_verbose":
        return .tooVerbose
    case "did_not_follow_instructions":
        return .didNotFollowInstructions
    case "incorrect":
        return .incorrect
    case "stereotype_or_bias":
        return .stereotypeOrBias
    case "suggestive_or_sexual":
        return .suggestiveOrSexual
    case "vulgar_or_offensive":
        return .vulgarOrOffensive
    case "triggered_guardrail_unexpectedly":
        return .triggeredGuardrailUnexpectedly
    default:
        return .unhelpful
    }
}
#endif

@_cdecl("fm_session_create_ex")
public func fm_session_create_ex(
    _ modelPtr: UnsafeMutableRawPointer?,
    _ instructionsJSON: UnsafePointer<CChar>?,
    _ transcriptJSON: UnsafePointer<CChar>?,
    _ toolsJSON: UnsafePointer<CChar>?,
    _ toolContext: UnsafeMutableRawPointer?,
    _ toolCallback: (@convention(c) (
        UnsafeMutableRawPointer?,
        UnsafePointer<CChar>?,
        UnsafePointer<CChar>?,
        UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?,
        UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
    ) -> Int32)?,
    _ errorOut: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutableRawPointer? {
    #if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
        do {
            let model = systemModel(from: modelPtr)
            let tools = try buildTools(
                specsJSON: toolsJSON.map { String(cString: $0) },
                context: toolContext,
                callback: toolCallback
            )
            let session: LanguageModelSession
            if let transcriptJSON {
                let transcript = try decodeTranscript(from: String(cString: transcriptJSON))
                session = LanguageModelSession(model: model, tools: tools, transcript: transcript)
            } else if let instructionsJSON {
                let instructionsBridge = try decodeBridge(String(cString: instructionsJSON), as: BridgeInstructions.self)
                let instructions = try buildInstructions(from: instructionsBridge)
                session = LanguageModelSession(model: model, tools: tools, instructions: instructions)
            } else {
                session = LanguageModelSession(model: model, tools: tools, instructions: nil)
            }
            return Unmanaged.passRetained(session).toOpaque()
        } catch {
            writeErrorOut(errorOut, error.localizedDescription)
            return nil
        }
    }
    #endif
    writeErrorOut(errorOut, "FoundationModels requires macOS 26.0 or newer")
    return nil
}

@_cdecl("fm_session_prewarm_prompt_json")
public func fm_session_prewarm_prompt_json(
    _ sessionPtr: UnsafeMutableRawPointer,
    _ promptJSON: UnsafePointer<CChar>?,
    _ errorOut: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    #if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
        do {
            let session = Unmanaged<LanguageModelSession>.fromOpaque(sessionPtr).takeUnretainedValue()
            let prompt = try promptJSON.map { try buildPrompt(from: decodeBridge(String(cString: $0), as: BridgePrompt.self)) }
            session.prewarm(promptPrefix: prompt)
            return FM_OK
        } catch {
            let (code, message) = mapError(error)
            writeErrorOut(errorOut, message)
            return code
        }
    }
    #endif
    writeErrorOut(errorOut, "FoundationModels requires macOS 26.0 or newer")
    return FM_MODEL_UNAVAILABLE
}

@_cdecl("fm_session_respond_request_json")
public func fm_session_respond_request_json(
    _ sessionPtr: UnsafeMutableRawPointer,
    _ requestJSON: UnsafePointer<CChar>,
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
        let requestJSONString = String(cString: requestJSON)
        Task.detached {
            do {
                let request = try decodeBridge(requestJSONString, as: BridgeResponseRequest.self)
                let prompt = try buildPrompt(from: request.prompt)
                let options = buildOptions(from: request.options)
                if let schemaJSON = request.schemaJSON {
                    let schema = try decodeGenerationSchema(from: schemaJSON)
                    let response = try await session.respond(
                        to: prompt,
                        schema: schema,
                        includeSchemaInPrompt: request.includeSchemaInPrompt ?? true,
                        options: options
                    )
                    let transcriptJSON = try encodeTranscriptJSON(entries: response.transcriptEntries)
                    let payload = BridgeStructuredResponse(
                        contentJSON: response.content.jsonString,
                        rawContentJSON: response.rawContent.jsonString,
                        transcriptJSON: transcriptJSON
                    )
                    callback(context, ffiString(try encodeBridge(payload)), nil, FM_OK)
                } else {
                    let response = try await session.respond(to: prompt, options: options)
                    let transcriptJSON = try encodeTranscriptJSON(entries: response.transcriptEntries)
                    let payload = BridgeTextResponse(
                        content: response.content,
                        rawContentJSON: response.rawContent.jsonString,
                        transcriptJSON: transcriptJSON
                    )
                    callback(context, ffiString(try encodeBridge(payload)), nil, FM_OK)
                }
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

@_cdecl("fm_session_stream_request_json")
public func fm_session_stream_request_json(
    _ sessionPtr: UnsafeMutableRawPointer,
    _ requestJSON: UnsafePointer<CChar>,
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
        let requestJSONString = String(cString: requestJSON)
        Task.detached {
            do {
                let request = try decodeBridge(requestJSONString, as: BridgeResponseRequest.self)
                let prompt = try buildPrompt(from: request.prompt)
                let options = buildOptions(from: request.options)
                if let schemaJSON = request.schemaJSON {
                    let schema = try decodeGenerationSchema(from: schemaJSON)
                    let stream = session.streamResponse(
                        to: prompt,
                        schema: schema,
                        includeSchemaInPrompt: request.includeSchemaInPrompt ?? true,
                        options: options
                    )
                    for try await snapshot in stream {
                        let payload = BridgeStructuredStreamSnapshot(
                            contentJSON: snapshot.content.jsonString,
                            rawContentJSON: snapshot.rawContent.jsonString,
                            isComplete: snapshot.content.isComplete
                        )
                        callback(context, ffiString(try encodeBridge(payload)), false, FM_OK)
                    }
                } else {
                    let stream = session.streamResponse(to: prompt, options: options)
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
                            rawContentJSON: snapshot.rawContent.jsonString
                        )
                        callback(context, ffiString(try encodeBridge(payload)), false, FM_OK)
                    }
                }
                callback(context, nil, true, FM_OK)
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

@_cdecl("fm_session_log_feedback_attachment_json")
public func fm_session_log_feedback_attachment_json(
    _ sessionPtr: UnsafeMutableRawPointer,
    _ requestJSON: UnsafePointer<CChar>,
    _ lengthOut: UnsafeMutablePointer<Int>?,
    _ errorOut: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutableRawPointer? {
    #if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
        do {
            let session = Unmanaged<LanguageModelSession>.fromOpaque(sessionPtr).takeUnretainedValue()
            let request = try decodeBridge(String(cString: requestJSON), as: BridgeFeedbackRequest.self)
            let issues = request.issues.map {
                LanguageModelFeedback.Issue(
                    category: feedbackIssueCategory(from: $0.category),
                    explanation: $0.explanation
                )
            }
            let sentiment = feedbackSentiment(from: request.sentiment)
            let data: Data
            if let desiredOutputTranscriptJSON = request.desiredOutputTranscriptJSON {
                data = session.logFeedbackAttachment(
                    sentiment: sentiment,
                    issues: issues,
                    desiredOutput: try firstTranscriptEntry(from: desiredOutputTranscriptJSON)
                )
            } else if let desiredResponseText = request.desiredResponseText {
                data = session.logFeedbackAttachment(
                    sentiment: sentiment,
                    issues: issues,
                    desiredResponseText: desiredResponseText
                )
            } else if let desiredResponseContentJSON = request.desiredResponseContentJSON {
                data = session.logFeedbackAttachment(
                    sentiment: sentiment,
                    issues: issues,
                    desiredResponseContent: try GeneratedContent(json: desiredResponseContentJSON)
                )
            } else {
                data = session.logFeedbackAttachment(
                    sentiment: sentiment,
                    issues: issues,
                    desiredOutput: nil
                )
            }
            lengthOut?.pointee = data.count
            return copyDataToHeap(data)
        } catch {
            writeErrorOut(errorOut, error.localizedDescription)
            return nil
        }
    }
    #endif
    writeErrorOut(errorOut, "FoundationModels requires macOS 26.0 or newer")
    return nil
}
