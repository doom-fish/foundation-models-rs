import Foundation

#if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
import FoundationModels
#endif

#if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
final class BridgeHandleTable<Value>: @unchecked Sendable {
    private let lock = NSLock()
    private let identity: (Value) -> AnyHashable?
    private var lastToken: UInt64 = 0
    private var entries: [UInt64: (value: Value, references: Int)] = [:]
    private var tokensByIdentity: [AnyHashable: UInt64] = [:]

    init(identity: @escaping (Value) -> AnyHashable? = { _ in nil }) {
        self.identity = identity
    }

    func insert(_ value: Value) -> UInt64 {
        let key = identity(value)
        lock.lock()
        defer { lock.unlock() }
        if let key, let token = tokensByIdentity[key], let entry = entries[token] {
            entries[token] = (entry.value, entry.references + 1)
            return token
        }
        lastToken += 1
        entries[lastToken] = (value, 1)
        if let key {
            tokensByIdentity[key] = lastToken
        }
        return lastToken
    }

    func retain(_ token: UInt64) -> Bool {
        lock.lock()
        defer { lock.unlock() }
        guard let entry = entries[token] else {
            return false
        }
        entries[token] = (entry.value, entry.references + 1)
        return true
    }

    func release(_ token: UInt64) {
        lock.lock()
        defer { lock.unlock() }
        guard let entry = entries[token] else {
            return
        }
        guard entry.references <= 1 else {
            entries[token] = (entry.value, entry.references - 1)
            return
        }
        entries[token] = nil
        if let key = identity(entry.value) {
            tokensByIdentity[key] = nil
        }
    }

    func value(for token: UInt64) -> Value? {
        lock.lock()
        defer { lock.unlock() }
        return entries[token]?.value
    }

    var count: Int {
        lock.lock()
        defer { lock.unlock() }
        return entries.count
    }
}

@available(macOS 26.0, *)
enum BridgeHandles {
    static let generationIDs = BridgeHandleTable<GenerationID>(identity: { AnyHashable($0) })
    static let refusals = BridgeHandleTable<LanguageModelSession.GenerationError.Refusal>()
}

@available(macOS 26.0, *)
final class BridgeLease {
    private var generationIDs: [UInt64] = []
    private var refusals: [UInt64] = []

    func lend(_ generationID: GenerationID) -> BridgeGenerationID {
        let token = BridgeHandles.generationIDs.insert(generationID)
        generationIDs.append(token)
        return BridgeGenerationID(token: token, description: String(describing: generationID))
    }

    func lend(_ refusal: LanguageModelSession.GenerationError.Refusal) -> BridgeRefusal {
        let token = BridgeHandles.refusals.insert(refusal)
        refusals.append(token)
        return BridgeRefusal(token: token)
    }

    func end() {
        generationIDs.forEach(BridgeHandles.generationIDs.release)
        refusals.forEach(BridgeHandles.refusals.release)
        generationIDs.removeAll()
        refusals.removeAll()
    }
}

@available(macOS 26.0, *)
extension BridgePrompt {
    var generationIDTokens: [UInt64] {
        segments.compactMap { $0.content?.generationID?.token }
    }
}
#endif

@_cdecl("fm_generation_id_retain")
public func fm_generation_id_retain(_ token: UInt64) -> Bool {
    #if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
        return BridgeHandles.generationIDs.retain(token)
    }
    #endif
    return false
}

@_cdecl("fm_generation_id_release")
public func fm_generation_id_release(_ token: UInt64) {
    #if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
        BridgeHandles.generationIDs.release(token)
    }
    #endif
}

@_cdecl("fm_refusal_retain")
public func fm_refusal_retain(_ token: UInt64) -> Bool {
    #if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
        return BridgeHandles.refusals.retain(token)
    }
    #endif
    return false
}

@_cdecl("fm_refusal_release")
public func fm_refusal_release(_ token: UInt64) {
    #if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
        BridgeHandles.refusals.release(token)
    }
    #endif
}

@_cdecl("fm_test_bridge_handle_counts")
public func fm_test_bridge_handle_counts(
    _ generationIDsOut: UnsafeMutablePointer<Int>?,
    _ refusalsOut: UnsafeMutablePointer<Int>?
) {
    #if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
        generationIDsOut?.pointee = BridgeHandles.generationIDs.count
        refusalsOut?.pointee = BridgeHandles.refusals.count
        return
    }
    #endif
    generationIDsOut?.pointee = 0
    refusalsOut?.pointee = 0
}

@_cdecl("fm_test_deliver_refusal")
public func fm_test_deliver_refusal(
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
        let lease = BridgeLease()
        defer { lease.end() }
        let error = LanguageModelSession.GenerationError.refusal(
            LanguageModelSession.GenerationError.Refusal(transcriptEntries: []),
            LanguageModelSession.GenerationError.Context(debugDescription: "test refusal")
        )
        let (code, message) = mapError(error, lease: lease)
        callback(context, nil, ffiString(message), code)
        return
    }
    #endif
    callback(context, nil, ffiString("FoundationModels requires macOS 26.0 or newer"), FM_MODEL_UNAVAILABLE)
}

@_cdecl("fm_test_deliver_identified_response")
public func fm_test_deliver_identified_response(
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
        let lease = BridgeLease()
        defer { lease.end() }
        do {
            let content = GeneratedContent("identified", id: GenerationID())
            let payload = BridgeTextResponse(
                content: "identified",
                rawContent: bridgeGeneratedContent(content, lease: lease),
                transcriptJSON: try encodeTranscriptJSON(entries: [])
            )
            callback(context, ffiString(try encodeBridge(payload)), nil, FM_OK)
        } catch {
            let (code, message) = mapError(error, lease: lease)
            callback(context, nil, ffiString(message), code)
        }
        return
    }
    #endif
    callback(context, nil, ffiString("FoundationModels requires macOS 26.0 or newer"), FM_MODEL_UNAVAILABLE)
}

@_cdecl("fm_test_accept_tool_output")
public func fm_test_accept_tool_output(_ outputJSON: UnsafePointer<CChar>) -> Int32 {
    #if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
        do {
            _ = try buildToolOutputPrompt(from: String(cString: outputJSON))
            return FM_OK
        } catch {
            return mapError(error).0
        }
    }
    #endif
    return FM_MODEL_UNAVAILABLE
}
