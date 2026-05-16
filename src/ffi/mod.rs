//! Raw FFI declarations matching the Swift `@_cdecl` exports in
//! `swift-bridge/Sources/FoundationModelsBridge`.
//!
//! These are intentionally opaque (`*mut c_void`) and unsafe; safe wrappers
//! live in the parent modules.

use core::ffi::{c_char, c_void};

/// Plain-old-data carrier for [`crate::generation::GenerationOptions`].
#[derive(Copy, Clone, Debug)]
pub struct FFIGenerationOptions {
    pub temperature: f64,
    pub maximum_response_tokens: i32,
    pub sampling_mode: i32,
    pub top_k: i32,
    pub top_p: f64,
    pub random_seed: u64,
    pub has_random_seed: bool,
}

pub type FmRespondCallback = unsafe extern "C" fn(
    context: *mut c_void,
    response: *mut c_char,
    error: *mut c_char,
    status: i32,
);

pub type FmStreamCallback =
    unsafe extern "C" fn(context: *mut c_void, chunk: *mut c_char, done: bool, status: i32);

pub type FmToolCallback = unsafe extern "C" fn(
    context: *mut c_void,
    tool_name: *const c_char,
    arguments_json: *const c_char,
    output_json_out: *mut *mut c_char,
    error_out: *mut *mut c_char,
) -> i32;

extern "C" {
    pub fn fm_string_dup(s: *const c_char) -> *mut c_char;
    pub fn fm_string_free(s: *mut c_char);
    pub fn fm_bytes_free(ptr: *mut c_void);
    pub fn fm_object_release(ptr: *mut c_void);

    pub fn fm_system_model_is_available() -> bool;
    pub fn fm_system_model_availability_code() -> i32;
    pub fn fm_system_model_create_default() -> *mut c_void;
    pub fn fm_system_model_create(
        use_case: i32,
        guardrails: i32,
        error_out: *mut *mut c_char,
    ) -> *mut c_void;
    pub fn fm_system_model_create_with_adapter(
        adapter: *mut c_void,
        guardrails: i32,
        error_out: *mut *mut c_char,
    ) -> *mut c_void;
    pub fn fm_system_model_availability_code_for(model: *mut c_void) -> i32;
    pub fn fm_system_model_supported_languages_json(model: *mut c_void) -> *mut c_char;
    pub fn fm_system_model_supports_locale(
        model: *mut c_void,
        locale_identifier: *const c_char,
    ) -> bool;

    pub fn fm_adapter_create_from_file(
        file_path: *const c_char,
        error_out: *mut *mut c_char,
    ) -> *mut c_void;
    pub fn fm_adapter_create_from_name(
        name: *const c_char,
        error_out: *mut *mut c_char,
    ) -> *mut c_void;
    pub fn fm_adapter_compile(
        adapter: *mut c_void,
        context: *mut c_void,
        callback: FmRespondCallback,
    );
    pub fn fm_adapter_compatible_identifiers_json(name: *const c_char) -> *mut c_char;
    pub fn fm_adapter_remove_obsolete(error_out: *mut *mut c_char) -> i32;
    pub fn fm_adapter_metadata_json(adapter: *mut c_void) -> *mut c_char;

    pub fn fm_session_create(instructions: *const c_char) -> *mut c_void;
    pub fn fm_session_create_ex(
        model: *mut c_void,
        instructions_json: *const c_char,
        transcript_json: *const c_char,
        tools_json: *const c_char,
        tool_context: *mut c_void,
        tool_callback: Option<FmToolCallback>,
        error_out: *mut *mut c_char,
    ) -> *mut c_void;

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
    pub fn fm_session_respond_request_json(
        session: *mut c_void,
        request_json: *const c_char,
        context: *mut c_void,
        callback: FmRespondCallback,
    );

    pub fn fm_session_respond_with_schema(
        session: *mut c_void,
        prompt: *const c_char,
        schema_json: *const c_char,
        include_schema_in_prompt: bool,
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
    pub fn fm_session_stream_request_json(
        session: *mut c_void,
        request_json: *const c_char,
        context: *mut c_void,
        callback: FmStreamCallback,
    );

    pub fn fm_session_prewarm(session: *mut c_void);
    pub fn fm_session_prewarm_prompt_json(
        session: *mut c_void,
        prompt_json: *const c_char,
        error_out: *mut *mut c_char,
    ) -> i32;
    pub fn fm_session_is_responding(session: *mut c_void) -> bool;
    pub fn fm_session_transcript_json(session: *mut c_void) -> *mut c_char;
    pub fn fm_session_log_feedback(
        session: *mut c_void,
        sentiment: i32,
        description: *const c_char,
    );
    pub fn fm_session_log_feedback_attachment_json(
        session: *mut c_void,
        request_json: *const c_char,
        length_out: *mut usize,
        error_out: *mut *mut c_char,
    ) -> *mut c_void;

    pub fn fm_generation_schema_compile_json(
        request_json: *const c_char,
        context: *mut c_void,
        callback: FmRespondCallback,
    );
    pub fn fm_generation_schema_validate_json(
        schema_json: *const c_char,
        error_out: *mut *mut c_char,
    ) -> i32;

    pub fn fm_generation_id_create(
        output_out: *mut *mut c_char,
        error_out: *mut *mut c_char,
    ) -> i32;
    pub fn fm_decimal_to_generated_content_json(
        decimal_string: *const c_char,
        output_out: *mut *mut c_char,
        error_out: *mut *mut c_char,
    ) -> i32;
    pub fn fm_decimal_from_generated_content_json(
        generated_content_json: *const c_char,
        output_out: *mut *mut c_char,
        error_out: *mut *mut c_char,
    ) -> i32;
    pub fn fm_refusal_explanation_json(
        refusal_token: *const c_char,
        context: *mut c_void,
        callback: FmRespondCallback,
    );
    pub fn fm_refusal_explanation_from_transcript_json(
        transcript_json: *const c_char,
        context: *mut c_void,
        callback: FmRespondCallback,
    );
    pub fn fm_refusal_explanation_stream(
        refusal_token: *const c_char,
        context: *mut c_void,
        callback: FmStreamCallback,
    );
    pub fn fm_refusal_explanation_stream_from_transcript_json(
        transcript_json: *const c_char,
        context: *mut c_void,
        callback: FmStreamCallback,
    );
}

/// Status codes mirrored 1:1 from the `FM_*` constants in Swift.
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
    pub const TOOL_CALL_FAILED: i32 = -13;
    pub const ADAPTER_INVALID_ASSET: i32 = -14;
    pub const ADAPTER_INVALID_NAME: i32 = -15;
    pub const ADAPTER_COMPATIBLE_NOT_FOUND: i32 = -16;
    pub const UNKNOWN: i32 = -99;
}
