// Async.swift — Tier-1 async thunks for FoundationModels APIs.
//
// Each function takes an opaque Rust context pointer and a C callback:
//
//   cb(ctx: UnsafeMutableRawPointer?, result: …?, error: UnsafeMutablePointer<CChar>?, status: Int32)
//
// On success  `status` is FM_OK and `result` is non-null.
// On failure  `result` is null and `error` is the heap-allocated error payload.
// Every pointer handed to the callback is heap-allocated and owned by the Rust caller:
//   • opaque object pointers → freed via `fm_object_release`
//   • strings                → freed via `fm_string_free`
// Each function returns a retained task handle (or NULL when the callback already ran);
// cancel it with `fm_task_cancel` and release it with `fm_object_release`.

import Foundation

#if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
import FoundationModels
#endif

// MARK: - Adapter init(name:) async

/// Async thunk for `SystemLanguageModel.Adapter(name:)`.
///
/// The returned object is an `Unmanaged.passRetained` AdapterBox; free it with
/// `fm_object_release`.
@_cdecl("fm_adapter_create_from_name_async")
public func fm_adapter_create_from_name_async(
    _ name: UnsafePointer<CChar>,
    _ ctx: UnsafeMutableRawPointer?,
    _ cb: @convention(c) (
        UnsafeMutableRawPointer?,
        UnsafeMutableRawPointer?,
        UnsafeMutablePointer<CChar>?,
        Int32
    ) -> Void
) -> UnsafeMutableRawPointer? {
    #if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
        let nameStr = String(cString: name)
        return startBridgeTask {
            do {
                try Task.checkCancellation()
                let adapter = try SystemLanguageModel.Adapter(name: nameStr)
                cb(ctx, Unmanaged.passRetained(AdapterBox(adapter)).toOpaque(), nil, FM_OK)
            } catch {
                let (code, message) = mapError(error)
                cb(ctx, nil, ffiString(message), code)
            }
        }
    }
    #endif
    cb(ctx, nil, ffiString("FoundationModels requires macOS 26.0 or newer"), FM_MODEL_UNAVAILABLE)
    return nil
}

// MARK: - Adapter.compatibility(for:) / compatibleAdapterIdentifiers async

/// Async thunk for `SystemLanguageModel.Adapter.compatibleAdapterIdentifiers(name:)`.
///
/// Note: the underlying Apple SDK call is synchronous as of macOS 26.0; it is
/// executed inside a detached Task so it runs off the caller's thread and fits
/// the async-Future pattern expected by the Rust `AsyncCompletion` machinery.
/// When Apple promotes this to `async throws`, the thunk will be updated.
@_cdecl("fm_adapter_compatibility_async")
public func fm_adapter_compatibility_async(
    _ name: UnsafePointer<CChar>,
    _ ctx: UnsafeMutableRawPointer?,
    _ cb: @convention(c) (
        UnsafeMutableRawPointer?,
        UnsafeMutablePointer<CChar>?,
        UnsafeMutablePointer<CChar>?,
        Int32
    ) -> Void
) -> UnsafeMutableRawPointer? {
    #if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
        let nameStr = String(cString: name)
        return startBridgeTask {
            do {
                try Task.checkCancellation()
                let ids = SystemLanguageModel.Adapter.compatibleAdapterIdentifiers(name: nameStr)
                cb(ctx, ffiString(try encodeBridge(ids)), nil, FM_OK)
            } catch {
                let (code, message) = mapError(error)
                cb(ctx, nil, ffiString(message), code)
            }
        }
    }
    #endif
    cb(ctx, nil, ffiString("FoundationModels requires macOS 26.0 or newer"), FM_MODEL_UNAVAILABLE)
    return nil
}
