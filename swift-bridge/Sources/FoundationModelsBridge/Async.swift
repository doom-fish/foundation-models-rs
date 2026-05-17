// Async.swift — Tier-1 async thunks for FoundationModels APIs.
//
// Each function takes an opaque Rust context pointer and a C callback with
// the 3-arg async pattern used by `doom_fish_utils::completion::AsyncCompletion`:
//
//   cb(result: UnsafeMutableRawPointer?, error: UnsafePointer<CChar>?, ctx: UnsafeMutableRawPointer?)
//
// On success  `result` is non-null, `error` is null.
// On failure  `result` is null,     `error` is a null-terminated UTF-8 error string.
// The `result` pointer is heap-allocated and must be freed by the Rust caller:
//   • opaque object pointers → freed via `fm_object_release`
//   • JSON string pointers   → freed via `fm_string_free`

import Foundation

#if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
import FoundationModels
#endif

// MARK: - Adapter init(name:) async

/// Async thunk for `SystemLanguageModel.Adapter(name:)`.
///
/// On success fires `cb(retainedAdapterPtr, nil, ctx)`.
/// The returned pointer is an `Unmanaged.passRetained` AdapterBox; free it with
/// `fm_object_release`.
@_cdecl("fm_adapter_create_from_name_async")
public func fm_adapter_create_from_name_async(
    _ name: UnsafePointer<CChar>,
    _ ctx: UnsafeMutableRawPointer?,
    _ cb: @convention(c) (
        UnsafeMutableRawPointer?,   // retained AdapterBox ptr  (success) or nil
        UnsafePointer<CChar>?,      // error C-string           (failure) or nil
        UnsafeMutableRawPointer?    // opaque Rust ctx
    ) -> Void
) {
    #if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
        let nameStr = String(cString: name)
        Task.detached {
            do {
                let adapter = try SystemLanguageModel.Adapter(name: nameStr)
                let ptr = Unmanaged.passRetained(AdapterBox(adapter)).toOpaque()
                cb(ptr, nil, ctx)
            } catch {
                error.localizedDescription.withCString { cb(nil, $0, ctx) }
            }
        }
        return
    }
    #endif
    "FoundationModels requires macOS 26.0 or newer".withCString { cb(nil, $0, ctx) }
}

// MARK: - Adapter.compatibility(for:) / compatibleAdapterIdentifiers async

/// Async thunk for `SystemLanguageModel.Adapter.compatibleAdapterIdentifiers(name:)`.
///
/// On success fires `cb(strdupJsonPtr, nil, ctx)` where the pointer is a
/// heap-allocated JSON array, e.g. `["com.example.adapter"]`; free it with
/// `fm_string_free`.
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
        UnsafeMutableRawPointer?,   // strdup JSON string ptr  (success) or nil
        UnsafePointer<CChar>?,      // error C-string          (failure) or nil
        UnsafeMutableRawPointer?    // opaque Rust ctx
    ) -> Void
) {
    #if canImport(FoundationModels) && FOUNDATION_MODELS_HAS_MACOS26_SDK
    if #available(macOS 26.0, *) {
        let nameStr = String(cString: name)
        Task.detached {
            do {
                let ids = SystemLanguageModel.Adapter.compatibleAdapterIdentifiers(name: nameStr)
                let json = try encodeBridge(ids)
                // strdup produces a heap-allocated copy that Rust must free via fm_string_free.
                if let dup = ffiString(json) {
                    cb(UnsafeMutableRawPointer(dup), nil, ctx)
                } else {
                    "Failed to allocate result string".withCString { cb(nil, $0, ctx) }
                }
            } catch {
                error.localizedDescription.withCString { cb(nil, $0, ctx) }
            }
        }
        return
    }
    #endif
    "FoundationModels requires macOS 26.0 or newer".withCString { cb(nil, $0, ctx) }
}
