import Foundation

#if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
import FoundationModels
#endif

typealias RustToolContextRelease = @convention(c) (UnsafeMutableRawPointer?) -> Void

final class RustToolContext: @unchecked Sendable {
    let pointer: UnsafeMutableRawPointer
    private let release: RustToolContextRelease

    init(pointer: UnsafeMutableRawPointer, release: @escaping RustToolContextRelease) {
        self.pointer = pointer
        self.release = release
    }

    deinit {
        release(pointer)
    }
}

let rustToolCallQueue = DispatchQueue(
    label: "foundation-models.tool-calls",
    qos: .userInitiated,
    attributes: .concurrent
)

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

    private let owner: RustToolContext
    private let callback: RustToolInvokeCallback

    init(
        spec: BridgeToolSpec,
        owner: RustToolContext,
        callback: @escaping RustToolInvokeCallback
    ) throws {
        name = spec.name
        description = spec.description
        parameters = try decodeGenerationSchema(from: spec.parametersJSON)
        includesSchemaInInstructions = spec.includesSchemaInInstructions
        self.owner = owner
        self.callback = callback
    }

    func call(arguments: GeneratedContent) async throws -> Prompt {
        let argumentsJSON = arguments.jsonString
        let name = self.name
        let owner = self.owner
        let callback = self.callback

        let (status, outputJSON, errorMessage) = await withCheckedContinuation {
            (continuation: CheckedContinuation<(Int32, String?, String?), Never>) in
            rustToolCallQueue.async {
                var outputJSONPtr: UnsafeMutablePointer<CChar>?
                var errorPtr: UnsafeMutablePointer<CChar>?
                let status = argumentsJSON.withCString { argumentsCString in
                    name.withCString { nameCString in
                        callback(owner.pointer, nameCString, argumentsCString, &outputJSONPtr, &errorPtr)
                    }
                }
                let outputJSON = outputJSONPtr.map { String(cString: $0) }
                let errorMessage = errorPtr.map { String(cString: $0) }
                if let outputJSONPtr {
                    fm_string_free(outputJSONPtr)
                }
                if let errorPtr {
                    fm_string_free(errorPtr)
                }
                continuation.resume(returning: (status, outputJSON, errorMessage))
            }
        }

        guard status == FM_OK else {
            throw NSError(domain: "fm-tool", code: Int(status), userInfo: [
                NSLocalizedDescriptionKey: errorMessage ?? "tool call failed"
            ])
        }

        guard let outputJSON else {
            throw NSError(domain: "fm-tool", code: Int(FM_TOOL_CALL_FAILED), userInfo: [
                NSLocalizedDescriptionKey: "tool callback returned success without an output"
            ])
        }

        let toolOutput = try decodeBridge(outputJSON, as: BridgeToolOutput.self)
        return try buildPrompt(from: toolOutput.prompt)
    }
}

@available(macOS 26.0, *)
func buildTools(
    specsJSON: String?,
    owner: RustToolContext?,
    callback: RustToolInvokeCallback?
) throws -> [any Tool] {
    guard let specsJSON, !specsJSON.isEmpty else {
        return []
    }
    guard let owner, let callback else {
        throw NSError(domain: "fm-bridge", code: Int(FM_INVALID_ARGUMENT), userInfo: [
            NSLocalizedDescriptionKey: "tool specs were provided without a tool context and callback"
        ])
    }
    let specs = try decodeBridge(specsJSON, as: [BridgeToolSpec].self)
    return try specs.map { try RustTool(spec: $0, owner: owner, callback: callback) }
}

#endif
