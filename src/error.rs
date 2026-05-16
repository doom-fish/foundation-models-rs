//! Errors produced by the `FoundationModels` bridge.

use core::ffi::c_char;
use core::fmt;

use crate::ffi;

/// Top-level error type returned by all fallible APIs in this crate.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FMError {
    /// `FoundationModels` is not available on this device.
    ///
    /// See [`Unavailability`] for the specific reason.
    ModelUnavailable {
        reason: Unavailability,
        message: String,
    },
    /// The model refused to produce a response because the prompt or
    /// generated content tripped a safety guardrail.
    GuardrailViolation(String),
    /// The combined prompt + history exceeds the model's context window.
    ContextWindowExceeded(String),
    /// The requested locale or language is not supported by the on-device model.
    UnsupportedLanguage(String),
    /// On-device model assets are still downloading or otherwise unavailable.
    AssetsUnavailable(String),
    /// The session was rate-limited (typically only relevant on Mac with
    /// extended generation budgets).
    RateLimited(String),
    /// Structured generation failed to decode the model's output into the
    /// requested `Generable` schema.
    DecodingFailure(String),
    /// The model refused the request (distinct from a guardrail violation —
    /// the model itself declined to answer).
    Refusal(String),
    /// Too many concurrent generation requests against the same session.
    ConcurrentRequests(String),
    /// The supplied [`GenerationGuide`] is unsupported by the on-device model.
    UnsupportedGuide(String),
    /// A tool invocation failed while the model was using `Tool` calling.
    ToolCallFailed(String),
    /// An adapter asset pack was invalid.
    AdapterInvalidAsset(String),
    /// The requested adapter name was invalid.
    AdapterInvalidName(String),
    /// No compatible adapter could be found for the requested name.
    AdapterCompatibleNotFound(String),
    /// The generation Task was cancelled before completion.
    Cancelled,
    /// An invalid argument crossed the FFI boundary (e.g. a NUL byte in a prompt).
    InvalidArgument(String),
    /// Catch-all for unmapped Swift errors. Inspect [`code`](Self::code) and
    /// [`message`](Self::message) for diagnostics.
    Unknown { code: i32, message: String },
}

/// Reason why [`SystemLanguageModel`](crate::SystemLanguageModel) is unavailable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Unavailability {
    /// The hardware does not support Apple Intelligence (e.g. Intel Mac, M1).
    DeviceNotEligible,
    /// Apple Intelligence is supported but disabled in System Settings.
    AppleIntelligenceNotEnabled,
    /// Model assets are still downloading.
    ModelNotReady,
    /// The host OS is older than macOS 26.0.
    OsTooOld,
    /// `FoundationModels` reported an unavailability reason this crate doesn't
    /// recognise — most likely added in a newer SDK.
    Unknown,
}

impl FMError {
    /// Numeric status code reported by the Swift bridge. Useful for matching
    /// against [`crate::ffi::status`] constants.
    #[must_use]
    pub const fn code(&self) -> i32 {
        match self {
            Self::ModelUnavailable { .. } => ffi::status::MODEL_UNAVAILABLE,
            Self::GuardrailViolation(_) => ffi::status::GUARDRAIL_VIOLATION,
            Self::ContextWindowExceeded(_) => ffi::status::CONTEXT_WINDOW_EXCEEDED,
            Self::UnsupportedLanguage(_) => ffi::status::UNSUPPORTED_LANGUAGE,
            Self::AssetsUnavailable(_) => ffi::status::ASSETS_UNAVAILABLE,
            Self::RateLimited(_) => ffi::status::RATE_LIMITED,
            Self::DecodingFailure(_) => ffi::status::DECODING_FAILURE,
            Self::Refusal(_) => ffi::status::REFUSAL,
            Self::ConcurrentRequests(_) => ffi::status::CONCURRENT_REQUESTS,
            Self::UnsupportedGuide(_) => ffi::status::UNSUPPORTED_GUIDE,
            Self::ToolCallFailed(_) => ffi::status::TOOL_CALL_FAILED,
            Self::AdapterInvalidAsset(_) => ffi::status::ADAPTER_INVALID_ASSET,
            Self::AdapterInvalidName(_) => ffi::status::ADAPTER_INVALID_NAME,
            Self::AdapterCompatibleNotFound(_) => ffi::status::ADAPTER_COMPATIBLE_NOT_FOUND,
            Self::Cancelled => ffi::status::CANCELLED,
            Self::InvalidArgument(_) => ffi::status::INVALID_ARGUMENT,
            Self::Unknown { code, .. } => *code,
        }
    }

    /// Human-readable description (forwarded from `Error.localizedDescription`).
    #[must_use]
    pub fn message(&self) -> &str {
        match self {
            Self::ModelUnavailable { message, .. }
            | Self::GuardrailViolation(message)
            | Self::ContextWindowExceeded(message)
            | Self::UnsupportedLanguage(message)
            | Self::AssetsUnavailable(message)
            | Self::RateLimited(message)
            | Self::DecodingFailure(message)
            | Self::Refusal(message)
            | Self::ConcurrentRequests(message)
            | Self::UnsupportedGuide(message)
            | Self::ToolCallFailed(message)
            | Self::AdapterInvalidAsset(message)
            | Self::AdapterInvalidName(message)
            | Self::AdapterCompatibleNotFound(message)
            | Self::InvalidArgument(message)
            | Self::Unknown { message, .. } => message,
            Self::Cancelled => "generation cancelled",
        }
    }
}

impl fmt::Display for FMError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (code {})", self.message(), self.code())
    }
}

impl std::error::Error for FMError {}

/// Build an `FMError` from a status code + error message returned by Swift.
///
/// Takes ownership of `error_str` (a heap-allocated C string from the
/// Swift bridge) and frees it via `fm_string_free` after copying.
pub(crate) fn from_swift(status: i32, error_str: *mut c_char) -> FMError {
    let message = if error_str.is_null() {
        String::new()
    } else {
        let s = unsafe { core::ffi::CStr::from_ptr(error_str) }
            .to_string_lossy()
            .into_owned();
        unsafe { ffi::fm_string_free(error_str) };
        s
    };

    match status {
        ffi::status::MODEL_UNAVAILABLE => FMError::ModelUnavailable {
            reason: Unavailability::Unknown,
            message,
        },
        ffi::status::GUARDRAIL_VIOLATION => FMError::GuardrailViolation(message),
        ffi::status::CONTEXT_WINDOW_EXCEEDED => FMError::ContextWindowExceeded(message),
        ffi::status::UNSUPPORTED_LANGUAGE => FMError::UnsupportedLanguage(message),
        ffi::status::ASSETS_UNAVAILABLE => FMError::AssetsUnavailable(message),
        ffi::status::RATE_LIMITED => FMError::RateLimited(message),
        ffi::status::DECODING_FAILURE => FMError::DecodingFailure(message),
        ffi::status::REFUSAL => FMError::Refusal(message),
        ffi::status::CONCURRENT_REQUESTS => FMError::ConcurrentRequests(message),
        ffi::status::UNSUPPORTED_GUIDE => FMError::UnsupportedGuide(message),
        ffi::status::TOOL_CALL_FAILED => FMError::ToolCallFailed(message),
        ffi::status::ADAPTER_INVALID_ASSET => FMError::AdapterInvalidAsset(message),
        ffi::status::ADAPTER_INVALID_NAME => FMError::AdapterInvalidName(message),
        ffi::status::ADAPTER_COMPATIBLE_NOT_FOUND => FMError::AdapterCompatibleNotFound(message),
        ffi::status::CANCELLED => FMError::Cancelled,
        ffi::status::INVALID_ARGUMENT => FMError::InvalidArgument(message),
        code => FMError::Unknown { code, message },
    }
}
