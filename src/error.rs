//! Errors produced by the `FoundationModels` bridge.

use core::ffi::c_char;
use core::fmt;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::sync::{Mutex, OnceLock};

use serde::Deserialize;

use crate::ffi;
use crate::prompt::ToolDefinition;
use crate::schema::GenerationSchema;
use crate::session::{self, SessionResponse, StreamEvent};
use crate::transcript::{Entry, Transcript};

/// Structured generation-error context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationErrorContext {
    debug_description: String,
}

impl GenerationErrorContext {
    /// Create a context object from a debug description string.
    #[must_use]
    pub fn new(debug_description: impl Into<String>) -> Self {
        Self {
            debug_description: debug_description.into(),
        }
    }

    /// Borrow the context's debug description.
    #[must_use]
    pub fn debug_description(&self) -> &str {
        &self.debug_description
    }
}

/// Structured schema-validation error context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaErrorContext {
    debug_description: String,
}

impl SchemaErrorContext {
    /// Create a context object from a debug description string.
    #[must_use]
    pub fn new(debug_description: impl Into<String>) -> Self {
        Self {
            debug_description: debug_description.into(),
        }
    }

    /// Borrow the context's debug description.
    #[must_use]
    pub fn debug_description(&self) -> &str {
        &self.debug_description
    }
}

/// Structured adapter-asset error context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterAssetErrorContext {
    debug_description: String,
}

impl AdapterAssetErrorContext {
    /// Create an adapter-asset error context from a debug description string.
    #[must_use]
    pub fn new(debug_description: impl Into<String>) -> Self {
        Self {
            debug_description: debug_description.into(),
        }
    }

    /// Borrow the context's debug description.
    #[must_use]
    pub fn debug_description(&self) -> &str {
        &self.debug_description
    }
}

/// Typed tool-call failure metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCallError {
    tool: ToolDefinition,
    underlying_error: String,
}

impl ToolCallError {
    /// Create a tool-call error from the tool definition and underlying error text.
    #[must_use]
    pub fn new(tool: ToolDefinition, underlying_error: impl Into<String>) -> Self {
        Self {
            tool,
            underlying_error: underlying_error.into(),
        }
    }

    /// Borrow the tool definition that failed.
    #[must_use]
    pub const fn tool(&self) -> &ToolDefinition {
        &self.tool
    }

    /// Borrow the underlying error text.
    #[must_use]
    pub fn underlying_error(&self) -> &str {
        &self.underlying_error
    }
}

/// Typed refusal helper returned by generation-refusal errors.
#[derive(Debug, Clone, PartialEq)]
pub struct Refusal {
    token: Option<String>,
    transcript: Option<Transcript>,
}

impl Refusal {
    /// Create a refusal helper from transcript entries.
    #[must_use]
    pub fn new(entries: impl IntoIterator<Item = Entry>) -> Self {
        Self {
            token: None,
            transcript: Some(Transcript::from_entries(entries.into_iter().collect())),
        }
    }

    pub(crate) fn from_token(token: impl Into<String>) -> Self {
        Self {
            token: Some(token.into()),
            transcript: None,
        }
    }

    /// Borrow the local transcript, if this refusal was constructed from entries.
    #[must_use]
    pub fn transcript(&self) -> Option<&Transcript> {
        self.transcript.as_ref()
    }

    /// Resolve the refusal's explanation response.
    ///
    /// # Errors
    ///
    /// Returns an [`FMError`] if the Swift bridge rejects the refusal helper.
    pub fn explanation(&self) -> Result<SessionResponse<String>, FMError> {
        if let Some(token) = &self.token {
            let token = CString::new(token.as_str()).map_err(|error| {
                FMError::InvalidArgument(format!(
                    "refusal token contains an interior NUL byte: {error}"
                ))
            })?;
            return session::request_text_response_with(|context, callback| unsafe {
                ffi::fm_refusal_explanation_json(token.as_ptr(), context, callback)
            });
        }

        let transcript = self.transcript.as_ref().ok_or_else(|| {
            FMError::InvalidArgument("refusal does not contain any transcript state".into())
        })?;
        let transcript_json = CString::new(transcript.to_json_string()?).map_err(|error| {
            FMError::InvalidArgument(format!(
                "refusal transcript JSON contains an interior NUL byte: {error}"
            ))
        })?;
        session::request_text_response_with(|context, callback| unsafe {
            ffi::fm_refusal_explanation_from_transcript_json(
                transcript_json.as_ptr(),
                context,
                callback,
            )
        })
    }

    /// Stream the refusal's explanation text.
    ///
    /// # Errors
    ///
    /// Returns an [`FMError`] if the Swift bridge rejects the refusal helper.
    pub fn explanation_stream<F>(&self, on_chunk: F) -> Result<(), FMError>
    where
        F: FnMut(StreamEvent<'_>) + Send + 'static,
    {
        if let Some(token) = &self.token {
            let token = CString::new(token.as_str()).map_err(|error| {
                FMError::InvalidArgument(format!(
                    "refusal token contains an interior NUL byte: {error}"
                ))
            })?;
            return session::run_text_stream_with(
                |context, callback| unsafe {
                    ffi::fm_refusal_explanation_stream(token.as_ptr(), context, callback)
                },
                on_chunk,
            );
        }

        let transcript = self.transcript.as_ref().ok_or_else(|| {
            FMError::InvalidArgument("refusal does not contain any transcript state".into())
        })?;
        let transcript_json = CString::new(transcript.to_json_string()?).map_err(|error| {
            FMError::InvalidArgument(format!(
                "refusal transcript JSON contains an interior NUL byte: {error}"
            ))
        })?;
        session::run_text_stream_with(
            |context, callback| unsafe {
                ffi::fm_refusal_explanation_stream_from_transcript_json(
                    transcript_json.as_ptr(),
                    context,
                    callback,
                )
            },
            on_chunk,
        )
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
struct ErrorMetadata {
    recovery_suggestion: Option<String>,
    failure_reason: Option<String>,
    generation_error_context: Option<GenerationErrorContext>,
    adapter_asset_error_context: Option<AdapterAssetErrorContext>,
    schema_error_context: Option<SchemaErrorContext>,
    refusal: Option<Refusal>,
    tool_call_error: Option<ToolCallError>,
}

#[derive(Debug, Deserialize)]
struct BridgeErrorContext {
    #[serde(rename = "debugDescription")]
    debug_description: String,
}

#[derive(Debug, Deserialize)]
struct BridgeRefusal {
    token: String,
}

#[derive(Debug, Deserialize)]
struct BridgeToolDefinition {
    name: String,
    description: String,
    #[serde(rename = "parametersJSON")]
    parameters_json: String,
}

#[derive(Debug, Deserialize)]
struct BridgeToolCallError {
    tool: BridgeToolDefinition,
    #[serde(rename = "underlyingError")]
    underlying_error: String,
}

#[derive(Debug, Deserialize)]
struct BridgeErrorPayload {
    message: String,
    #[serde(rename = "recoverySuggestion")]
    recovery_suggestion: Option<String>,
    #[serde(rename = "failureReason")]
    failure_reason: Option<String>,
    #[serde(rename = "generationErrorContext")]
    generation_error_context: Option<BridgeErrorContext>,
    refusal: Option<BridgeRefusal>,
    #[serde(rename = "toolCallError")]
    tool_call_error: Option<BridgeToolCallError>,
    #[serde(rename = "adapterAssetErrorContext")]
    adapter_asset_error_context: Option<BridgeErrorContext>,
    #[serde(rename = "schemaErrorContext")]
    schema_error_context: Option<BridgeErrorContext>,
}

impl BridgeErrorPayload {
    fn into_metadata(self) -> ErrorMetadata {
        ErrorMetadata {
            recovery_suggestion: self.recovery_suggestion,
            failure_reason: self.failure_reason,
            generation_error_context: self
                .generation_error_context
                .map(|context| GenerationErrorContext::new(context.debug_description)),
            adapter_asset_error_context: self
                .adapter_asset_error_context
                .map(|context| AdapterAssetErrorContext::new(context.debug_description)),
            schema_error_context: self
                .schema_error_context
                .map(|context| SchemaErrorContext::new(context.debug_description)),
            refusal: self
                .refusal
                .map(|refusal| Refusal::from_token(refusal.token)),
            tool_call_error: self.tool_call_error.map(|error| {
                ToolCallError::new(
                    ToolDefinition::new(
                        error.tool.name,
                        error.tool.description,
                        GenerationSchema::from_json_schema_unchecked(error.tool.parameters_json),
                    ),
                    error.underlying_error,
                )
            }),
        }
    }
}

fn metadata_registry() -> &'static Mutex<HashMap<usize, ErrorMetadata>> {
    static REGISTRY: OnceLock<Mutex<HashMap<usize, ErrorMetadata>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn register_metadata(message: &str, metadata: ErrorMetadata) {
    if metadata == ErrorMetadata::default() {
        return;
    }
    metadata_registry()
        .lock()
        .expect("error metadata registry mutex poisoned")
        .insert(message.as_ptr() as usize, metadata);
}

fn clone_message_with_metadata(message: &str) -> String {
    let cloned = message.to_owned();
    let metadata = metadata_registry()
        .lock()
        .expect("error metadata registry mutex poisoned")
        .get(&(message.as_ptr() as usize))
        .cloned();
    if let Some(metadata) = metadata {
        register_metadata(&cloned, metadata);
    }
    cloned
}

/// Top-level error type returned by all fallible APIs in this crate.
#[derive(Debug, PartialEq, Eq)]
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
    /// The supplied [`crate::schema::GenerationGuide`] is unsupported by the on-device model.
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

impl Clone for FMError {
    fn clone(&self) -> Self {
        match self {
            Self::ModelUnavailable { reason, message } => Self::ModelUnavailable {
                reason: *reason,
                message: clone_message_with_metadata(message),
            },
            Self::GuardrailViolation(message) => {
                Self::GuardrailViolation(clone_message_with_metadata(message))
            }
            Self::ContextWindowExceeded(message) => {
                Self::ContextWindowExceeded(clone_message_with_metadata(message))
            }
            Self::UnsupportedLanguage(message) => {
                Self::UnsupportedLanguage(clone_message_with_metadata(message))
            }
            Self::AssetsUnavailable(message) => {
                Self::AssetsUnavailable(clone_message_with_metadata(message))
            }
            Self::RateLimited(message) => Self::RateLimited(clone_message_with_metadata(message)),
            Self::DecodingFailure(message) => {
                Self::DecodingFailure(clone_message_with_metadata(message))
            }
            Self::Refusal(message) => Self::Refusal(clone_message_with_metadata(message)),
            Self::ConcurrentRequests(message) => {
                Self::ConcurrentRequests(clone_message_with_metadata(message))
            }
            Self::UnsupportedGuide(message) => {
                Self::UnsupportedGuide(clone_message_with_metadata(message))
            }
            Self::ToolCallFailed(message) => {
                Self::ToolCallFailed(clone_message_with_metadata(message))
            }
            Self::AdapterInvalidAsset(message) => {
                Self::AdapterInvalidAsset(clone_message_with_metadata(message))
            }
            Self::AdapterInvalidName(message) => {
                Self::AdapterInvalidName(clone_message_with_metadata(message))
            }
            Self::AdapterCompatibleNotFound(message) => {
                Self::AdapterCompatibleNotFound(clone_message_with_metadata(message))
            }
            Self::Cancelled => Self::Cancelled,
            Self::InvalidArgument(message) => {
                Self::InvalidArgument(clone_message_with_metadata(message))
            }
            Self::Unknown { code, message } => Self::Unknown {
                code: *code,
                message: clone_message_with_metadata(message),
            },
        }
    }
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
    fn message_storage(&self) -> Option<&String> {
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
            | Self::Unknown { message, .. } => Some(message),
            Self::Cancelled => None,
        }
    }

    fn metadata(&self) -> Option<ErrorMetadata> {
        let message = self.message_storage()?;
        metadata_registry()
            .lock()
            .expect("error metadata registry mutex poisoned")
            .get(&(message.as_ptr() as usize))
            .cloned()
    }

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

    /// Structured generation-error context, when available.
    #[must_use]
    pub fn generation_error_context(&self) -> Option<GenerationErrorContext> {
        self.metadata()?.generation_error_context
    }

    /// Structured adapter-asset error context, when available.
    #[must_use]
    pub fn adapter_asset_error_context(&self) -> Option<AdapterAssetErrorContext> {
        self.metadata()?.adapter_asset_error_context
    }

    /// Structured schema-error context, when available.
    #[must_use]
    pub fn schema_error_context(&self) -> Option<SchemaErrorContext> {
        self.metadata()?.schema_error_context
    }

    /// Localized recovery suggestion, when the SDK provided one.
    #[must_use]
    pub fn recovery_suggestion(&self) -> Option<String> {
        self.metadata()?.recovery_suggestion
    }

    /// Localized failure reason, when the SDK provided one.
    #[must_use]
    pub fn failure_reason(&self) -> Option<String> {
        self.metadata()?.failure_reason
    }

    /// Typed refusal helper, when this error came from a refusal.
    #[must_use]
    pub fn refusal(&self) -> Option<Refusal> {
        self.metadata()?.refusal
    }

    /// Typed tool-call failure metadata, when available.
    #[must_use]
    pub fn tool_call_error(&self) -> Option<ToolCallError> {
        self.metadata()?.tool_call_error
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
    let raw_message = if error_str.is_null() {
        String::new()
    } else {
        let value = unsafe { CStr::from_ptr(error_str) }
            .to_string_lossy()
            .into_owned();
        unsafe { ffi::fm_string_free(error_str) };
        value
    };

    let (message, metadata) = match serde_json::from_str::<BridgeErrorPayload>(&raw_message) {
        Ok(payload) => {
            let message = payload.message.clone();
            let metadata = payload.into_metadata();
            (message, Some(metadata))
        }
        Err(_) => (raw_message, None),
    };

    let error = match status {
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
    };

    if let (Some(message), Some(metadata)) = (error.message_storage(), metadata) {
        register_metadata(message, metadata);
    }

    error
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn payload_ptr(value: serde_json::Value) -> *mut c_char {
        let payload = CString::new(value.to_string()).expect("JSON payloads must not contain NUL");
        unsafe { ffi::fm_string_dup(payload.as_ptr()) }
    }

    #[test]
    fn generation_error_metadata_round_trips() {
        let error = from_swift(
            ffi::status::REFUSAL,
            payload_ptr(json!({
                "message": "request refused",
                "recoverySuggestion": "Try a safer prompt",
                "failureReason": "Safety policy",
                "generationErrorContext": { "debugDescription": "guardrail refusal" },
                "refusal": { "token": "refusal-token" }
            })),
        );
        let cloned = error.clone();

        assert_eq!(error.recovery_suggestion(), cloned.recovery_suggestion());
        assert_eq!(cloned.message(), "request refused");
        assert_eq!(
            cloned.recovery_suggestion().as_deref(),
            Some("Try a safer prompt")
        );
        assert_eq!(cloned.failure_reason().as_deref(), Some("Safety policy"));
        assert_eq!(
            cloned
                .generation_error_context()
                .expect("generation context")
                .debug_description(),
            "guardrail refusal"
        );
        assert_eq!(cloned.refusal(), Some(Refusal::from_token("refusal-token")));
    }

    #[test]
    fn tool_call_error_metadata_round_trips() {
        let error = from_swift(
            ffi::status::TOOL_CALL_FAILED,
            payload_ptr(json!({
                "message": "tool failed",
                "toolCallError": {
                    "tool": {
                        "name": "echo",
                        "description": "Echo input",
                        "parametersJSON": "{\"type\":\"object\"}"
                    },
                    "underlyingError": "callback panicked"
                }
            })),
        );

        let tool_call_error = error.tool_call_error().expect("tool call metadata");
        assert_eq!(tool_call_error.tool().name, "echo");
        assert_eq!(tool_call_error.tool().description, "Echo input");
        assert_eq!(
            tool_call_error.tool().parameters.json_schema(),
            "{\"type\":\"object\"}"
        );
        assert_eq!(tool_call_error.underlying_error(), "callback panicked");
    }

    #[test]
    fn schema_error_metadata_round_trips() {
        let error = from_swift(
            ffi::status::UNKNOWN,
            payload_ptr(json!({
                "message": "schema rejected",
                "recoverySuggestion": "Rename the duplicate type",
                "schemaErrorContext": { "debugDescription": "duplicate type Person" }
            })),
        );

        assert_eq!(
            error.recovery_suggestion().as_deref(),
            Some("Rename the duplicate type")
        );
        assert_eq!(
            error
                .schema_error_context()
                .expect("schema context")
                .debug_description(),
            "duplicate type Person"
        );
    }

    #[test]
    fn adapter_asset_error_metadata_round_trips() {
        let error = from_swift(
            ffi::status::ADAPTER_INVALID_NAME,
            payload_ptr(json!({
                "message": "adapter not found",
                "recoverySuggestion": "Install a compatible adapter first",
                "adapterAssetErrorContext": { "debugDescription": "missing adapter metadata" }
            })),
        );

        assert_eq!(
            error.recovery_suggestion().as_deref(),
            Some("Install a compatible adapter first")
        );
        assert_eq!(
            error
                .adapter_asset_error_context()
                .expect("adapter asset context")
                .debug_description(),
            "missing adapter metadata"
        );
    }
}
