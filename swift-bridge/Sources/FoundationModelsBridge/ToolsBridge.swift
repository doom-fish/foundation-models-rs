import Foundation

#if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
import FoundationModels
#endif

#if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
@available(macOS 26.0, *)
typealias RustToolInvokeCallback = @convention(c) (
    UnsafeMutableRawPointer?,
    UnsafePointer<CChar>?,
    UnsafePointer<CChar>?,
    UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?,
    UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32

@available(macOS 26.0, *)
final class RustTool: Tool, @unchecked Sendable {
    typealias Arguments = GeneratedContent
    typealias Output = Prompt

    let name: String
    let description: String
    let parameters: GenerationSchema
    let includesSchemaInInstructions: Bool

    private let context: UnsafeMutableRawPointer?
    private let callback: RustToolInvokeCallback

    init(
        spec: BridgeToolSpec,
        context: UnsafeMutableRawPointer?,
        callback: @escaping RustToolInvokeCallback
    ) throws {
        name = spec.name
        description = spec.description
        parameters = try decodeGenerationSchema(from: spec.parametersJSON)
        includesSchemaInInstructions = spec.includesSchemaInInstructions
        self.context = context
        self.callback = callback
    }

    func call(arguments: GeneratedContent) async throws -> Prompt {
        var outputJSONPtr: UnsafeMutablePointer<CChar>?
        var errorPtr: UnsafeMutablePointer<CChar>?

        let status = arguments.jsonString.withCString { argumentsCString in
            name.withCString { nameCString in
                callback(context, nameCString, argumentsCString, &outputJSONPtr, &errorPtr)
            }
        }

        defer {
            if let outputJSONPtr {
                fm_string_free(outputJSONPtr)
            }
            if let errorPtr {
                fm_string_free(errorPtr)
            }
        }

        guard status == FM_OK else {
            let message = errorPtr.map { String(cString: $0) } ?? "tool call failed"
            throw NSError(domain: "fm-tool", code: Int(status), userInfo: [
                NSLocalizedDescriptionKey: message
            ])
        }

        guard let outputJSONPtr else {
            throw NSError(domain: "fm-tool", code: Int(FM_TOOL_CALL_FAILED), userInfo: [
                NSLocalizedDescriptionKey: "tool callback returned success without an output"
            ])
        }

        let outputJSON = String(cString: outputJSONPtr)
        let toolOutput = try decodeBridge(outputJSON, as: BridgeToolOutput.self)
        return try buildPrompt(from: toolOutput.prompt)
    }
}

@available(macOS 26.0, *)
func buildTools(
    specsJSON: String?,
    context: UnsafeMutableRawPointer?,
    callback: RustToolInvokeCallback?
) throws -> [any Tool] {
    guard let specsJSON, !specsJSON.isEmpty else {
        return []
    }
    guard let callback else {
        throw NSError(domain: "fm-tool", code: Int(FM_INVALID_ARGUMENT), userInfo: [
            NSLocalizedDescriptionKey: "tool specs were provided without a callback"
        ])
    }
    let specs = try decodeBridge(specsJSON, as: [BridgeToolSpec].self)
    return try specs.map { try RustTool(spec: $0, context: context, callback: callback) }
}
#endif
