//! Raw FFI declarations matching the Swift `@_cdecl` exports in
//! `swift-bridge/Sources/FoundationModelsBridge`.
//!
//! These are intentionally opaque (`*mut c_void`) and unsafe; safe wrappers
//! live in the parent modules.

use core::ffi::{c_char, c_void};

/// Plain-old-data carrier for [`crate::GenerationOptions`], lowered into
/// scalar arguments at the Swift FFI boundary because `@_cdecl` cannot
/// accept Swift struct pointer parameters.
///
/// `temperature == NaN` signals "leave default";
/// `maximum_response_tokens == 0` signals "no limit".
#[derive(Copy, Clone, Debug)]
pub struct FFIGenerationOptions {
    pub temperature: f64,
    pub maximum_response_tokens: i32,
    pub sampling_mode: i32,
    pub top_k: i32,
    pub top_p: f64,
}

pub type FmRespondCallback = unsafe extern "C" fn(
    context: *mut c_void,
    response: *mut c_char,
    error: *mut c_char,
    status: i32,
);

pub type FmStreamCallback =
    unsafe extern "C" fn(context: *mut c_void, chunk: *mut c_char, done: bool, status: i32);

extern "C" {
    pub fn fm_string_dup(s: *const c_char) -> *mut c_char;
    pub fn fm_string_free(s: *mut c_char);
    pub fn fm_object_release(ptr: *mut c_void);

    pub fn fm_system_model_is_available() -> bool;
    pub fn fm_system_model_availability_code() -> i32;

    pub fn fm_session_create(instructions: *const c_char) -> *mut c_void;

    pub fn fm_session_respond(
        session: *mut c_void,
        prompt: *const c_char,
        temperature: f64,
        max_tokens: i32,
        sampling_mode: i32,
        top_k: i32,
        top_p: f64,
        context: *mut c_void,
        callback: FmRespondCallback,
    );

    pub fn fm_session_stream_response(
        session: *mut c_void,
        prompt: *const c_char,
        temperature: f64,
        max_tokens: i32,
        sampling_mode: i32,
        top_k: i32,
        top_p: f64,
        context: *mut c_void,
        callback: FmStreamCallback,
    );

    /// Pre-warm the model. Apple loads weights + initialises the
    /// inference engine so the next `respond` call is faster.
    pub fn fm_session_prewarm(session: *mut c_void);

    /// Returns `true` if `session` is currently producing a response.
    pub fn fm_session_is_responding(session: *mut c_void) -> bool;
}

/// Status codes mirrored 1:1 from the `FM_*` constants in
/// `swift-bridge/Sources/FoundationModelsBridge/FoundationModels.swift`.
pub mod status {
    pub const OK: i32 = 0;
    pub const INVALID_ARGUMENT: i32 = -1;
    pub const MODEL_UNAVAILABLE: i32 = -2;
    pub const CANCELLED: i32 = -3;
    pub const GUARDRAIL_VIOLATION: i32 = -4;
    pub const CONTEXT_WINDOW_EXCEEDED: i32 = -5;
    pub const UNSUPPORTED_LANGUAGE: i32 = -6;
    pub const ASSETS_UNAVAILABLE: i32 = -7;
    pub const RATE_LIMITED: i32 = -8;
    pub const DECODING_FAILURE: i32 = -9;
    pub const REFUSAL: i32 = -10;
    pub const CONCURRENT_REQUESTS: i32 = -11;
    pub const UNSUPPORTED_GUIDE: i32 = -12;
    pub const UNKNOWN: i32 = -99;
}
