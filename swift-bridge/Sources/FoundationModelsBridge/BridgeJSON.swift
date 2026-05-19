import Foundation

#if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
import FoundationModels
#endif

struct BridgeGenerationID: Codable {
    var token: String
    var description: String
}

struct BridgeGeneratedContent: Codable {
    var json: String
    var generationID: BridgeGenerationID?
}

struct BridgeRefusal: Codable {
    var token: String
}

struct BridgeSegment: Codable {
    enum Kind: String, Codable {
        case text
        case structure
    }

    var kind: Kind
    var text: String?
    var source: String?
    var content: BridgeGeneratedContent?
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
    let rawContent: BridgeGeneratedContent
    let transcriptJSON: String
}

struct BridgeStructuredResponse: Encodable {
    let kind: String = "generated_content"
    let content: BridgeGeneratedContent
    let rawContent: BridgeGeneratedContent
    let transcriptJSON: String
}

struct BridgeTextStreamSnapshot: Encodable {
    let kind: String = "text"
    let delta: String
    let content: String
    let rawContent: BridgeGeneratedContent
}

struct BridgeStructuredStreamSnapshot: Encodable {
    let kind: String = "generated_content"
    let content: BridgeGeneratedContent
    let rawContent: BridgeGeneratedContent
    let isComplete: Bool
}

struct BridgeToolSpec: Codable {
    var name: String
    var description: String
    var parametersJSON: String
    var includesSchemaInInstructions: Bool
}

struct BridgeToolDefinition: Codable {
    var name: String
    var description: String
    var parametersJSON: String
}

struct BridgeToolCallErrorPayload: Codable {
    var tool: BridgeToolDefinition
    var underlyingError: String
}

struct BridgeErrorContext: Codable {
    var debugDescription: String
}

struct BridgeErrorPayload: Codable {
    var message: String
    var recoverySuggestion: String?
    var failureReason: String?
    var generationErrorContext: BridgeErrorContext?
    var refusal: BridgeRefusal?
    var toolCallError: BridgeToolCallErrorPayload?
    var adapterAssetErrorContext: BridgeErrorContext?
    var schemaErrorContext: BridgeErrorContext?
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
    var desiredResponseContent: BridgeGeneratedContent?
    var desiredOutputTranscriptJSON: String?
}

#if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
@available(macOS 26.0, *)
final class GenerationIDRegistry {
    static let shared = GenerationIDRegistry()

    private let lock = NSLock()
    private var generationIDs: [String: GenerationID] = [:]

    func register(_ generationID: GenerationID) -> BridgeGenerationID {
        lock.lock()
        defer { lock.unlock() }
        let token = UUID().uuidString
        generationIDs[token] = generationID
        return BridgeGenerationID(token: token, description: String(describing: generationID))
    }

    func resolve(_ bridgeGenerationID: BridgeGenerationID?) -> GenerationID? {
        guard let bridgeGenerationID else { return nil }
        lock.lock()
        defer { lock.unlock() }
        return generationIDs[bridgeGenerationID.token]
    }
}

@available(macOS 26.0, *)
final class RefusalRegistry {
    static let shared = RefusalRegistry()

    private let lock = NSLock()
    private var refusals: [String: LanguageModelSession.GenerationError.Refusal] = [:]

    func register(_ refusal: LanguageModelSession.GenerationError.Refusal) -> BridgeRefusal {
        lock.lock()
        defer { lock.unlock() }
        let token = UUID().uuidString
        refusals[token] = refusal
        return BridgeRefusal(token: token)
    }

    func resolve(_ bridgeRefusal: BridgeRefusal) -> LanguageModelSession.GenerationError.Refusal? {
        lock.lock()
        defer { lock.unlock() }
        return refusals[bridgeRefusal.token]
    }
}

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
func encodeErrorPayload(_ payload: BridgeErrorPayload) -> String {
    (try? encodeBridge(payload)) ?? payload.message
}

@available(macOS 26.0, *)
func bridgeGenerationID(from generationID: GenerationID?) -> BridgeGenerationID? {
    generationID.map(GenerationIDRegistry.shared.register)
}

@available(macOS 26.0, *)
func bridgeGeneratedContent(_ content: GeneratedContent) -> BridgeGeneratedContent {
    BridgeGeneratedContent(
        json: content.jsonString,
        generationID: bridgeGenerationID(from: content.id)
    )
}

@available(macOS 26.0, *)
func buildGeneratedContent(from bridge: BridgeGeneratedContent) throws -> GeneratedContent {
    let content = try GeneratedContent(json: bridge.json)
    if let generationID = GenerationIDRegistry.shared.resolve(bridge.generationID) {
        return GeneratedContent(content, id: generationID)
    }
    return content
}

@available(macOS 26.0, *)
func bridgeRefusal(_ refusal: LanguageModelSession.GenerationError.Refusal) -> BridgeRefusal {
    RefusalRegistry.shared.register(refusal)
}

@available(macOS 26.0, *)
func bridgeToolDefinition(from tool: any Tool) -> BridgeToolDefinition {
    BridgeToolDefinition(
        name: tool.name,
        description: tool.description,
        parametersJSON: (try? encodeBridge(tool.parameters)) ?? "{}"
    )
}

@available(macOS 26.0, *)
func buildPrompt(from bridge: BridgePrompt) throws -> Prompt {
    let parts = try bridge.segments.map { segment -> Prompt in
        switch segment.kind {
        case .text:
            return Prompt(segment.text ?? "")
        case .structure:
            guard let content = segment.content else {
                throw NSError(domain: "fm-bridge", code: Int(FM_INVALID_ARGUMENT), userInfo: [
                    NSLocalizedDescriptionKey: "structured prompt segment is missing content"
                ])
            }
            return Prompt(try buildGeneratedContent(from: content))
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
            guard let content = segment.content else {
                throw NSError(domain: "fm-bridge", code: Int(FM_INVALID_ARGUMENT), userInfo: [
                    NSLocalizedDescriptionKey: "structured instructions segment is missing content"
                ])
            }
            return Instructions(try buildGeneratedContent(from: content))
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
    do {
        return try JSONDecoder().decode(GenerationSchema.self, from: data)
    } catch {
        guard let parsed = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else {
            throw error
        }
        if let rootJSON = parsed["root"] {
            let root = try bridgeBuildDynamicSchema(from: rootJSON, name: "Root")
            let dependencies = try ((parsed["dependencies"] as? [Any]) ?? []).enumerated().map { index, value in
                try bridgeBuildDynamicSchema(from: value, name: "Dependency\(index)")
            }
            return try GenerationSchema(root: root, dependencies: dependencies)
        }
        if parsed["properties"] is [Any] || parsed["representNilExplicitlyInGeneratedContent"] != nil {
            let properties = try ((parsed["properties"] as? [Any]) ?? []).map { try typedSchemaProperty(from: $0) }
            let description = parsed["description"] as? String
            let explicitNil = (parsed["representNilExplicitlyInGeneratedContent"] as? Bool) ?? false
            if explicitNil {
                if #available(macOS 26.4, *) {
                    return GenerationSchema(
                        type: GeneratedContent.self,
                        description: description,
                        representNilExplicitlyInGeneratedContent: true,
                        properties: properties
                    )
                }
                throw schemaBridgeError("explicit nil representation requires macOS 26.4 or newer")
            }
            return GenerationSchema(type: GeneratedContent.self, description: description, properties: properties)
        }
        throw error
    }
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
