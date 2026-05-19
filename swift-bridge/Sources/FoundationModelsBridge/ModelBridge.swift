import Foundation

#if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
import FoundationModels
#endif

#if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
@available(macOS 26.0, *)
final class AdapterBox {
    let value: SystemLanguageModel.Adapter

    init(_ value: SystemLanguageModel.Adapter) {
        self.value = value
    }
}

@available(macOS 26.0, *)
func systemModelAvailabilityCode(for model: SystemLanguageModel) -> Int32 {
    switch model.availability {
    case .available:
        return 0
    case .unavailable(let reason):
        switch reason {
        case .deviceNotEligible:
            return 1
        case .appleIntelligenceNotEnabled:
            return 2
        case .modelNotReady:
            return 3
        @unknown default:
            return 4
        }
    @unknown default:
        return 4
    }
}

@available(macOS 26.0, *)
func systemModel(from ptr: UnsafeMutableRawPointer?) -> SystemLanguageModel {
    guard let ptr else {
        return .default
    }
    return Unmanaged<SystemLanguageModel>.fromOpaque(ptr).takeUnretainedValue()
}

@available(macOS 26.0, *)
func adapter(from ptr: UnsafeMutableRawPointer) -> SystemLanguageModel.Adapter {
    Unmanaged<AdapterBox>.fromOpaque(ptr).takeUnretainedValue().value
}

@available(macOS 26.0, *)
func modelUseCase(from rawValue: Int32) -> SystemLanguageModel.UseCase {
    switch rawValue {
    case 1:
        return .contentTagging
    default:
        return .general
    }
}

@available(macOS 26.0, *)
func modelGuardrails(from rawValue: Int32) -> SystemLanguageModel.Guardrails {
    switch rawValue {
    case 1:
        return .permissiveContentTransformations
    default:
        return .default
    }
}

@available(macOS 26.0, *)
func jsonSafeObject(_ value: Any) -> Any {
    switch value {
    case let dict as [String: Any]:
        return dict.mapValues(jsonSafeObject)
    case let array as [Any]:
        return array.map(jsonSafeObject)
    case let number as NSNumber:
        return number
    case let string as String:
        return string
    case let date as Date:
        return ISO8601DateFormatter().string(from: date)
    case _ as NSNull:
        return NSNull()
    default:
        return String(describing: value)
    }
}
#endif

@_cdecl("fm_system_model_create_default")
public func fm_system_model_create_default() -> UnsafeMutableRawPointer? {
    #if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
        return Unmanaged.passRetained(SystemLanguageModel.default).toOpaque()
    }
    #endif
    return nil
}

@_cdecl("fm_system_model_create")
public func fm_system_model_create(
    _ useCase: Int32,
    _ guardrails: Int32,
    _ errorOut: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutableRawPointer? {
    #if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
        let model = SystemLanguageModel(
            useCase: modelUseCase(from: useCase),
            guardrails: modelGuardrails(from: guardrails)
        )
        return Unmanaged.passRetained(model).toOpaque()
    }
    #endif
    writeErrorOut(errorOut, "FoundationModels requires macOS 26.0 or newer")
    return nil
}

@_cdecl("fm_system_model_create_with_adapter")
public func fm_system_model_create_with_adapter(
    _ adapterPtr: UnsafeMutableRawPointer?,
    _ guardrails: Int32,
    _ errorOut: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutableRawPointer? {
    #if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
        guard let adapterPtr else {
            writeErrorOut(errorOut, "adapter pointer must not be null")
            return nil
        }
        let model = SystemLanguageModel(
            adapter: adapter(from: adapterPtr),
            guardrails: modelGuardrails(from: guardrails)
        )
        return Unmanaged.passRetained(model).toOpaque()
    }
    #endif
    writeErrorOut(errorOut, "FoundationModels requires macOS 26.0 or newer")
    return nil
}

@_cdecl("fm_system_model_availability_code_for")
public func fm_system_model_availability_code_for(_ modelPtr: UnsafeMutableRawPointer?) -> Int32 {
    #if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
        return systemModelAvailabilityCode(for: systemModel(from: modelPtr))
    }
    #endif
    return -1
}

@_cdecl("fm_system_model_supported_languages_json")
public func fm_system_model_supported_languages_json(
    _ modelPtr: UnsafeMutableRawPointer?
) -> UnsafeMutablePointer<CChar>? {
    #if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
        let model = systemModel(from: modelPtr)
        let languages = model.supportedLanguages
            .map(\.maximalIdentifier)
            .sorted()
        return ffiString((try? encodeBridge(languages)) ?? "[]")
    }
    #endif
    return ffiString("[]")
}

@_cdecl("fm_system_model_supports_locale")
public func fm_system_model_supports_locale(
    _ modelPtr: UnsafeMutableRawPointer?,
    _ localeIdentifier: UnsafePointer<CChar>?
) -> Bool {
    #if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
        let model = systemModel(from: modelPtr)
        let locale = localeIdentifier.map { Locale(identifier: String(cString: $0)) } ?? Locale.current
        return model.supportsLocale(locale)
    }
    #endif
    return false
}

@_cdecl("fm_system_model_token_count_prompt_async")
public func fm_system_model_token_count_prompt_async(
    _ modelPtr: UnsafeMutableRawPointer?,
    _ prompt: UnsafePointer<CChar>,
    _ ctx: UnsafeMutableRawPointer?,
    _ cb: @convention(c) (
        UnsafeMutableRawPointer?,
        UnsafePointer<CChar>?,
        UnsafeMutableRawPointer?
    ) -> Void
) {
    #if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
    if #available(macOS 26.4, *) {
        let model = systemModel(from: modelPtr)
        let promptValue = Prompt(String(cString: prompt))
        Task.detached {
            do {
                let count = try await model.tokenCount(for: promptValue)
                if let dup = ffiString(String(count)) {
                    cb(UnsafeMutableRawPointer(dup), nil, ctx)
                } else {
                    "Failed to allocate token-count result".withCString { cb(nil, $0, ctx) }
                }
            } catch {
                let (_, message) = mapError(error)
                message.withCString { cb(nil, $0, ctx) }
            }
        }
        return
    }
    #endif
    "FoundationModels token count requires macOS 26.4 or newer".withCString { cb(nil, $0, ctx) }
}

@_cdecl("fm_adapter_create_from_file")
public func fm_adapter_create_from_file(
    _ filePath: UnsafePointer<CChar>,
    _ errorOut: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutableRawPointer? {
    #if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
        do {
            let raw = String(cString: filePath)
            let url = raw.hasPrefix("file:") ? URL(string: raw)! : URL(fileURLWithPath: raw)
            let adapter = try SystemLanguageModel.Adapter(fileURL: url)
            return Unmanaged.passRetained(AdapterBox(adapter)).toOpaque()
        } catch {
            writeErrorOut(errorOut, error.localizedDescription)
            return nil
        }
    }
    #endif
    writeErrorOut(errorOut, "FoundationModels requires macOS 26.0 or newer")
    return nil
}

@_cdecl("fm_adapter_create_from_name")
public func fm_adapter_create_from_name(
    _ name: UnsafePointer<CChar>,
    _ errorOut: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutableRawPointer? {
    #if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
        do {
            let adapter = try SystemLanguageModel.Adapter(name: String(cString: name))
            return Unmanaged.passRetained(AdapterBox(adapter)).toOpaque()
        } catch {
            writeErrorOut(errorOut, error.localizedDescription)
            return nil
        }
    }
    #endif
    writeErrorOut(errorOut, "FoundationModels requires macOS 26.0 or newer")
    return nil
}

@_cdecl("fm_adapter_compile")
public func fm_adapter_compile(
    _ adapterPtr: UnsafeMutableRawPointer,
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
        let adapter = adapter(from: adapterPtr)
        Task.detached {
            do {
                try await adapter.compile()
                callback(context, ffiString("ok"), nil, FM_OK)
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

@_cdecl("fm_adapter_compatible_identifiers_json")
public func fm_adapter_compatible_identifiers_json(
    _ name: UnsafePointer<CChar>
) -> UnsafeMutablePointer<CChar>? {
    #if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
        let identifiers = SystemLanguageModel.Adapter.compatibleAdapterIdentifiers(name: String(cString: name))
        return ffiString((try? encodeBridge(identifiers)) ?? "[]")
    }
    #endif
    return ffiString("[]")
}

@_cdecl("fm_adapter_remove_obsolete")
public func fm_adapter_remove_obsolete(
    _ errorOut: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    #if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
        do {
            try SystemLanguageModel.Adapter.removeObsoleteAdapters()
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

@_cdecl("fm_adapter_metadata_json")
public func fm_adapter_metadata_json(_ adapterPtr: UnsafeMutableRawPointer) -> UnsafeMutablePointer<CChar>? {
    #if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
        let metadata = jsonSafeObject(adapter(from: adapterPtr).creatorDefinedMetadata)
        if JSONSerialization.isValidJSONObject(metadata),
           let data = try? JSONSerialization.data(withJSONObject: metadata, options: []),
           let string = String(data: data, encoding: .utf8) {
            return ffiString(string)
        }
        return ffiString("{}")
    }
    #endif
    return ffiString("{}")
}
