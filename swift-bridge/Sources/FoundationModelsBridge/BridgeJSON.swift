import Foundation

#if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
import FoundationModels
#endif

struct BridgeSegment: Codable {
    enum Kind: String, Codable {
        case text
        case structure
    }

    var kind: Kind
    var text: String?
    var source: String?
    var contentJSON: String?
}

struct BridgePrompt: Codable {
    var segments: [BridgeSegment]
}

typealias BridgeInstructions = BridgePrompt

struct BridgeSampling: Codable {
    var mode: String
    var topK: Int?
    var topP: Double?
    var seed: UInt64?
}

struct BridgeGenerationOptions: Codable {
    var temperature: Double?
    var maximumResponseTokens: Int?
    var sampling: BridgeSampling?
}

struct BridgeResponseRequest: Codable {
    var prompt: BridgePrompt
    var options: BridgeGenerationOptions?
    var schemaJSON: String?
    var includeSchemaInPrompt: Bool?
}

struct BridgeTextResponse: Encodable {
    let kind: String = "text"
    let content: String
    let rawContentJSON: String
    let transcriptJSON: String
}

struct BridgeStructuredResponse: Encodable {
    let kind: String = "generated_content"
    let contentJSON: String
    let rawContentJSON: String
    let transcriptJSON: String
}

struct BridgeTextStreamSnapshot: Encodable {
    let kind: String = "text"
    let delta: String
    let content: String
    let rawContentJSON: String
}

struct BridgeStructuredStreamSnapshot: Encodable {
    let kind: String = "generated_content"
    let contentJSON: String
    let rawContentJSON: String
    let isComplete: Bool
}

struct BridgeToolSpec: Codable {
    var name: String
    var description: String
    var parametersJSON: String
    var includesSchemaInInstructions: Bool
}

struct BridgeToolOutput: Codable {
    var prompt: BridgePrompt
}

struct BridgeFeedbackIssue: Codable {
    var category: String
    var explanation: String?
}

struct BridgeFeedbackRequest: Codable {
    var sentiment: String?
    var issues: [BridgeFeedbackIssue]
    var desiredResponseText: String?
    var desiredResponseContentJSON: String?
    var desiredOutputTranscriptJSON: String?
}

#if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
@available(macOS 26.0, *)
func decodeBridge<T: Decodable>(_ json: String, as type: T.Type = T.self) throws -> T {
    guard let data = json.data(using: .utf8) else {
        throw NSError(domain: "fm-bridge", code: Int(FM_INVALID_ARGUMENT), userInfo: [
            NSLocalizedDescriptionKey: "input is not valid UTF-8"
        ])
    }
    return try JSONDecoder().decode(T.self, from: data)
}

@available(macOS 26.0, *)
func encodeBridge<T: Encodable>(_ value: T) throws -> String {
    let encoder = JSONEncoder()
    let data = try encoder.encode(value)
    guard let string = String(data: data, encoding: .utf8) else {
        throw NSError(domain: "fm-bridge", code: Int(FM_UNKNOWN), userInfo: [
            NSLocalizedDescriptionKey: "failed to encode JSON as UTF-8"
        ])
    }
    return string
}

@available(macOS 26.0, *)
func buildPrompt(from bridge: BridgePrompt) throws -> Prompt {
    let parts = try bridge.segments.map { segment -> Prompt in
        switch segment.kind {
        case .text:
            return Prompt(segment.text ?? "")
        case .structure:
            guard let contentJSON = segment.contentJSON else {
                throw NSError(domain: "fm-bridge", code: Int(FM_INVALID_ARGUMENT), userInfo: [
                    NSLocalizedDescriptionKey: "structured prompt segment is missing contentJSON"
                ])
            }
            let content = try GeneratedContent(json: contentJSON)
            return Prompt(content)
        }
    }
    return parts.isEmpty ? Prompt("") : Prompt(parts)
}

@available(macOS 26.0, *)
func buildInstructions(from bridge: BridgeInstructions) throws -> Instructions {
    let parts = try bridge.segments.map { segment -> Instructions in
        switch segment.kind {
        case .text:
            return Instructions(segment.text ?? "")
        case .structure:
            guard let contentJSON = segment.contentJSON else {
                throw NSError(domain: "fm-bridge", code: Int(FM_INVALID_ARGUMENT), userInfo: [
                    NSLocalizedDescriptionKey: "structured instructions segment is missing contentJSON"
                ])
            }
            let content = try GeneratedContent(json: contentJSON)
            return Instructions(content)
        }
    }
    return parts.isEmpty ? Instructions("") : Instructions(parts)
}

@available(macOS 26.0, *)
func buildOptions(from bridge: BridgeGenerationOptions?) -> GenerationOptions {
    guard let bridge else {
        return GenerationOptions()
    }

    var sampling: GenerationOptions.SamplingMode?
    switch bridge.sampling?.mode {
    case nil, "default":
        sampling = nil
    case "greedy":
        sampling = .greedy
    case "top_k":
        sampling = .random(top: bridge.sampling?.topK ?? 1, seed: bridge.sampling?.seed)
    case "top_p":
        sampling = .random(
            probabilityThreshold: bridge.sampling?.topP ?? 1.0,
            seed: bridge.sampling?.seed
        )
    default:
        sampling = nil
    }

    return GenerationOptions(
        sampling: sampling,
        temperature: bridge.temperature,
        maximumResponseTokens: bridge.maximumResponseTokens
    )
}

@available(macOS 26.0, *)
func decodeGenerationSchema(from json: String) throws -> GenerationSchema {
    guard let data = json.data(using: .utf8) else {
        throw NSError(domain: "fm-bridge", code: Int(FM_INVALID_ARGUMENT), userInfo: [
            NSLocalizedDescriptionKey: "schema JSON is not valid UTF-8"
        ])
    }
    return try JSONDecoder().decode(GenerationSchema.self, from: data)
}

@available(macOS 26.0, *)
func encodeTranscriptJSON<S>(entries: S) throws -> String where S: Sequence, S.Element == Transcript.Entry {
    let transcript = Transcript(entries: entries)
    return try encodeBridge(transcript)
}

@available(macOS 26.0, *)
func decodeTranscript(from json: String) throws -> Transcript {
    try decodeBridge(json, as: Transcript.self)
}

@available(macOS 26.0, *)
func firstTranscriptEntry(from json: String) throws -> Transcript.Entry {
    let transcript = try decodeTranscript(from: json)
    guard let entry = transcript.first else {
        throw NSError(domain: "fm-bridge", code: Int(FM_INVALID_ARGUMENT), userInfo: [
            NSLocalizedDescriptionKey: "transcript JSON does not contain any entries"
        ])
    }
    return entry
}
#endif
