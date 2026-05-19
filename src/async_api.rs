//! Executor-agnostic async API for `FoundationModels` (Tier 1).
//!
//! Enabled with the `async` Cargo feature.  Works with any async runtime
//! (Tokio, async-std, smol, pollster, …) because it uses only `std` types
//! internally.
//!
//! ## Wrapped Apple APIs
//!
//! | Rust type | Apple API | Notes |
//! |-----------|-----------|-------|
//! | [`AsyncSession::respond`] | `LanguageModelSession.respond(to:)` | Returns `SessionResponse<String>` |
//! | [`AsyncSession::respond_generating`] | `LanguageModelSession.respond(to:generating:)` | Returns `SessionResponse<GeneratedContent>` |
//! | [`AsyncAdapter::from_name`] | `SystemLanguageModel.Adapter init(name:)` | Returns `Adapter` |
//! | [`AsyncAdapter::compatibility`] | `SystemLanguageModel.Adapter.compatibility(for:)` | Returns `Vec<String>` |
//!
//! ## Tier 2 note
//!
//! `LanguageModelSession.streamResponse(to:)` is an `AsyncSequence` — a
//! multi-fire stream, not a one-shot future.  It is deferred to **Tier 2**
//! (stream pattern).  Use [`crate::LanguageModelSession::stream`] for
//! synchronous streaming in the meantime.
//!
//! ## Example
//!
//! ```rust,no_run
//! use foundation_models::{LanguageModelSession, SystemLanguageModel};
//! use foundation_models::async_api::AsyncSession;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! if !SystemLanguageModel::is_available() {
//!     eprintln!("SKIP: FoundationModels unavailable");
//!     return Ok(());
//! }
//! pollster::block_on(async {
//!     let session = LanguageModelSession::new();
//!     let async_session = AsyncSession::new(&session);
//!     let reply = async_session.respond("Name three Norse gods.")?.await?;
//!     println!("{}", reply.content);
//!     Ok::<(), Box<dyn std::error::Error>>(())
//! })
//! # }
//! ```

use std::ffi::{c_void, CStr, CString};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use doom_fish_utils::completion::{error_from_cstr, AsyncCompletion, AsyncCompletionFuture};
use serde::Deserialize;

use crate::content::{BridgeGeneratedContent, GeneratedContent};
use crate::error::FMError;
use crate::ffi;
use crate::generation::GenerationOptions;
use crate::model::Adapter;
use crate::prompt::{Prompt, ToPrompt};
use crate::schema::GenerationSchema;
use crate::session::{decode_bridge_text_response, SessionResponse};
use crate::transcript::Transcript;

// ============================================================================
// Private bridge structs – mirror the JSON shapes emitted by SessionExtras.swift
// ============================================================================

#[derive(Debug, Deserialize)]
struct AsyncBridgeStructuredResponse {
    content: BridgeGeneratedContent,
    #[serde(rename = "rawContent")]
    raw_content: BridgeGeneratedContent,
    #[serde(rename = "transcriptJSON")]
    transcript_json: String,
}

// ============================================================================
// Opaque pointer newtype – needed so AsyncCompletion<OpaquePtr> is Send
// ============================================================================

/// Thin Send-able wrapper around a raw opaque pointer returned by Swift.
///
/// # Safety
///
/// The pointer is a retained `AdapterBox` produced by
/// `Unmanaged.passRetained(…).toOpaque()` on the Swift side.  We only
/// ever pass it back to `fm_object_release`; we never dereference it in
/// Rust.  Swift's reference counting is thread-safe, so `Send` is valid.
struct OpaquePtr(*mut c_void);
// SAFETY: See doc comment above.
unsafe impl Send for OpaquePtr {}

// ============================================================================
// Callback: `FmRespondCallback` (4-arg) → AsyncCompletion<String>
//
// Reuses the existing `fm_session_respond_request_json` FFI which already
// runs `try await session.respond(…)` inside a Swift Task.
// ============================================================================

/// Async respond callback.  Matches `ffi::FmRespondCallback`.
///
/// On success copies the JSON response to an owned `String` and completes
/// the `AsyncCompletion`.  On failure maps the status + error to an
/// `FMError` message string (the Future newtypes re-map that to `FMError`).
///
/// # Safety
///
/// `ctx` must be a valid `AsyncCompletion<String>` context pointer.
/// `response` and `error` are nullable C strings owned by the Swift bridge.
unsafe extern "C" fn respond_async_cb(
    ctx: *mut c_void,
    response: *mut std::ffi::c_char,
    error: *mut std::ffi::c_char,
    status: i32,
) {
    if status == ffi::status::OK && !response.is_null() {
        let s = unsafe { CStr::from_ptr(response) }
            .to_string_lossy()
            .into_owned();
        unsafe { ffi::fm_string_free(response) };
        unsafe { AsyncCompletion::complete_ok(ctx, s) };
    } else {
        // Re-use the existing from_swift error mapper; convert FMError to String
        // so we can store it in AsyncCompletion<String>.
        let fm_err = crate::error::from_swift(status, error);
        unsafe { AsyncCompletion::<String>::complete_err(ctx, fm_err.to_string()) };
    }
}

// ============================================================================
// Callback: 3-arg async callback → AsyncCompletion<OpaquePtr>
//
// Used by fm_adapter_create_from_name_async.
// ============================================================================

/// # Safety
///
/// `ctx` must be a valid `AsyncCompletion<OpaquePtr>` context pointer.
unsafe extern "C" fn adapter_init_async_cb(
    result: *mut c_void,
    error: *const std::ffi::c_char,
    ctx: *mut c_void,
) {
    if !error.is_null() {
        let msg = unsafe { error_from_cstr(error) };
        unsafe { AsyncCompletion::<OpaquePtr>::complete_err(ctx, msg) };
    } else if !result.is_null() {
        unsafe { AsyncCompletion::complete_ok(ctx, OpaquePtr(result)) };
    } else {
        unsafe { AsyncCompletion::<OpaquePtr>::complete_err(ctx, "null adapter pointer".into()) };
    }
}

// ============================================================================
// Callback: 3-arg async callback → AsyncCompletion<String>
//
// Used by fm_adapter_compatibility_async.  The result pointer is a strdup'd
// JSON string; we copy it and free it.
// ============================================================================

/// # Safety
///
/// `ctx` must be a valid `AsyncCompletion<String>` context pointer.
/// `result` (when non-null) must be a heap-allocated C string freed with
/// `fm_string_free`.
unsafe extern "C" fn adapter_compat_async_cb(
    result: *mut c_void,
    error: *const std::ffi::c_char,
    ctx: *mut c_void,
) {
    if !error.is_null() {
        let msg = unsafe { error_from_cstr(error) };
        unsafe { AsyncCompletion::<String>::complete_err(ctx, msg) };
    } else if !result.is_null() {
        let s = unsafe { CStr::from_ptr(result.cast::<std::ffi::c_char>()) }
            .to_string_lossy()
            .into_owned();
        // Free the strdup'd JSON string allocated by the Swift bridge.
        unsafe { ffi::fm_string_free(result.cast::<std::ffi::c_char>()) };
        unsafe { AsyncCompletion::complete_ok(ctx, s) };
    } else {
        unsafe { AsyncCompletion::<String>::complete_err(ctx, "null compatibility result".into()) };
    }
}

// ============================================================================
// RespondFuture — LanguageModelSession.respond(to:)
// ============================================================================

/// Future returned by [`AsyncSession::respond`].
///
/// Resolves to `Result<SessionResponse<String>, FMError>`.
pub struct RespondFuture {
    inner: AsyncCompletionFuture<String>,
}

impl std::fmt::Debug for RespondFuture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RespondFuture").finish_non_exhaustive()
    }
}

impl Future for RespondFuture {
    type Output = Result<SessionResponse<String>, FMError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.inner).poll(cx).map(|r| {
            r.map_err(|msg| FMError::Unknown {
                code: ffi::status::UNKNOWN,
                message: msg,
            })
            .and_then(|json| decode_bridge_text_response(&json))
        })
    }
}

// ============================================================================
// RespondGeneratingFuture — LanguageModelSession.respond(to:generating:)
// ============================================================================

/// Future returned by [`AsyncSession::respond_generating`].
///
/// Resolves to `Result<SessionResponse<GeneratedContent>, FMError>`.
pub struct RespondGeneratingFuture {
    inner: AsyncCompletionFuture<String>,
}

impl std::fmt::Debug for RespondGeneratingFuture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RespondGeneratingFuture")
            .finish_non_exhaustive()
    }
}

impl Future for RespondGeneratingFuture {
    type Output = Result<SessionResponse<GeneratedContent>, FMError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.inner).poll(cx).map(|r| {
            r.map_err(|msg| FMError::Unknown {
                code: ffi::status::UNKNOWN,
                message: msg,
            })
            .and_then(|json| {
                let response: AsyncBridgeStructuredResponse = serde_json::from_str(&json)
                    .map_err(|e| FMError::DecodingFailure(e.to_string()))?;
                Ok(SessionResponse {
                    content: GeneratedContent::from_bridge_payload(response.content, true)?,
                    raw_content: GeneratedContent::from_bridge_payload(response.raw_content, true)?,
                    transcript: Transcript::from_json_str(&response.transcript_json)?,
                })
            })
        })
    }
}

// ============================================================================
// AdapterInitFuture — SystemLanguageModel.Adapter init(name:)
// ============================================================================

/// Future returned by [`AsyncAdapter::from_name`].
///
/// Resolves to `Result<Adapter, FMError>`.
pub struct AdapterInitFuture {
    inner: AsyncCompletionFuture<OpaquePtr>,
}

impl std::fmt::Debug for AdapterInitFuture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdapterInitFuture").finish_non_exhaustive()
    }
}

impl Future for AdapterInitFuture {
    type Output = Result<Adapter, FMError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.inner).poll(cx).map(|r| {
            r.map_err(FMError::AdapterInvalidName)
                .map(|OpaquePtr(ptr)| Adapter { ptr })
        })
    }
}

// ============================================================================
// AdapterCompatibilityFuture — SystemLanguageModel.Adapter.compatibility(for:)
// ============================================================================

/// Future returned by [`AsyncAdapter::compatibility`].
///
/// Resolves to `Result<Vec<String>, FMError>`.
pub struct AdapterCompatibilityFuture {
    inner: AsyncCompletionFuture<String>,
}

impl std::fmt::Debug for AdapterCompatibilityFuture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdapterCompatibilityFuture")
            .finish_non_exhaustive()
    }
}

impl Future for AdapterCompatibilityFuture {
    type Output = Result<Vec<String>, FMError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.inner).poll(cx).map(|r| {
            r.map_err(FMError::AdapterCompatibleNotFound)
                .and_then(|json| {
                    serde_json::from_str::<Vec<String>>(&json)
                        .map_err(|e| FMError::DecodingFailure(e.to_string()))
                })
        })
    }
}

// ============================================================================
// AsyncSession — async wrapper around LanguageModelSession
// ============================================================================

/// Async wrapper around [`crate::LanguageModelSession`].
///
/// Borrows the session for its lifetime; the session itself must outlive all
/// in-flight futures.
///
/// # Examples
///
/// ```rust,no_run
/// use foundation_models::{LanguageModelSession, SystemLanguageModel};
/// use foundation_models::async_api::AsyncSession;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// if !SystemLanguageModel::is_available() { return Ok(()); }
/// pollster::block_on(async {
///     let session = LanguageModelSession::new();
///     let reply = AsyncSession::new(&session).respond("Hi!")?.await?;
///     println!("{}", reply.content);
///     Ok::<(), Box<dyn std::error::Error>>(())
/// })
/// # }
/// ```
pub struct AsyncSession<'s> {
    session: &'s crate::session::LanguageModelSession,
}

impl<'s> AsyncSession<'s> {
    /// Wrap a [`crate::LanguageModelSession`] for async use.
    #[must_use]
    pub fn new(session: &'s crate::session::LanguageModelSession) -> Self {
        Self { session }
    }

    /// Async version of `LanguageModelSession.respond(to:)`.
    ///
    /// Corresponds to the Swift `async throws` method
    /// `LanguageModelSession.respond(to:)`.
    ///
    /// # Errors
    ///
    /// Returns an [`FMError`] if the model is unavailable or generation fails.
    pub fn respond(&self, prompt: impl ToPrompt) -> Result<RespondFuture, FMError> {
        let prompt = prompt.to_prompt()?;
        let payload = build_text_request_json(&prompt, GenerationOptions::new())?;
        let session_ptr = self.session.as_ptr();
        let (future, ctx) = AsyncCompletion::create();
        unsafe {
            ffi::fm_session_respond_request_json(
                session_ptr,
                payload.as_ptr(),
                ctx,
                respond_async_cb,
            );
        }
        Ok(RespondFuture { inner: future })
    }

    /// Async version of `LanguageModelSession.respond(to:)` with [`GenerationOptions`].
    ///
    /// # Errors
    ///
    /// Returns an [`FMError`] if the model is unavailable or generation fails.
    pub fn respond_with_options(
        &self,
        prompt: impl ToPrompt,
        options: GenerationOptions,
    ) -> Result<RespondFuture, FMError> {
        let prompt = prompt.to_prompt()?;
        let payload = build_text_request_json(&prompt, options)?;
        let session_ptr = self.session.as_ptr();
        let (future, ctx) = AsyncCompletion::create();
        unsafe {
            ffi::fm_session_respond_request_json(
                session_ptr,
                payload.as_ptr(),
                ctx,
                respond_async_cb,
            );
        }
        Ok(RespondFuture { inner: future })
    }

    /// Async version of `LanguageModelSession.respond(to:generating:)`.
    ///
    /// Generates a structured `GeneratedContent` response according to
    /// `schema`.  Corresponds to the Swift `async throws` method
    /// `LanguageModelSession.respond(to:generating:)`.
    ///
    /// # Errors
    ///
    /// Returns an [`FMError`] if the model is unavailable or generation fails.
    pub fn respond_generating(
        &self,
        prompt: impl ToPrompt,
        schema: &GenerationSchema,
        include_schema_in_prompt: bool,
        options: GenerationOptions,
    ) -> Result<RespondGeneratingFuture, FMError> {
        let prompt = prompt.to_prompt()?;
        let payload =
            build_structured_request_json(&prompt, options, schema, include_schema_in_prompt)?;
        let session_ptr = self.session.as_ptr();
        let (future, ctx) = AsyncCompletion::create();
        unsafe {
            ffi::fm_session_respond_request_json(
                session_ptr,
                payload.as_ptr(),
                ctx,
                respond_async_cb,
            );
        }
        Ok(RespondGeneratingFuture { inner: future })
    }
}

impl std::fmt::Debug for AsyncSession<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AsyncSession").finish_non_exhaustive()
    }
}

// ============================================================================
// AsyncAdapter — async adapter lifecycle
// ============================================================================

/// Namespace for async [`Adapter`] operations.
///
/// # Examples
///
/// ```rust,no_run
/// use foundation_models::async_api::AsyncAdapter;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// pollster::block_on(async {
///     let ids = AsyncAdapter::compatibility("com.example.MyAdapter")?.await?;
///     println!("compatible: {ids:?}");
///     Ok::<(), Box<dyn std::error::Error>>(())
/// })
/// # }
/// ```
pub struct AsyncAdapter;

impl AsyncAdapter {
    /// Async version of `SystemLanguageModel.Adapter init(name:)`.
    ///
    /// Loads the named adapter asynchronously, returning a ready-to-use
    /// [`Adapter`] handle.
    ///
    /// # Errors
    ///
    /// Returns an [`FMError::AdapterInvalidName`] if the adapter is not found
    /// or the name contains a NUL byte.
    pub fn from_name(name: &str) -> Result<AdapterInitFuture, FMError> {
        let cname = CString::new(name)
            .map_err(|e| FMError::InvalidArgument(format!("NUL byte in adapter name: {e}")))?;
        let (future, ctx) = AsyncCompletion::create();
        unsafe {
            ffi::fm_adapter_create_from_name_async(cname.as_ptr(), ctx, adapter_init_async_cb);
        }
        Ok(AdapterInitFuture { inner: future })
    }

    /// Async version of `SystemLanguageModel.Adapter.compatibility(for:)`.
    ///
    /// Returns the list of compatible adapter identifiers for the given
    /// logical adapter name.
    ///
    /// # Errors
    ///
    /// Returns an [`FMError::AdapterCompatibleNotFound`] on failure.
    pub fn compatibility(name: &str) -> Result<AdapterCompatibilityFuture, FMError> {
        let cname = CString::new(name)
            .map_err(|e| FMError::InvalidArgument(format!("NUL byte in adapter name: {e}")))?;
        let (future, ctx) = AsyncCompletion::create();
        unsafe {
            ffi::fm_adapter_compatibility_async(cname.as_ptr(), ctx, adapter_compat_async_cb);
        }
        Ok(AdapterCompatibilityFuture { inner: future })
    }
}

// ============================================================================
// Internal JSON request builders
// ============================================================================

fn build_text_request_json(
    prompt: &Prompt,
    options: GenerationOptions,
) -> Result<CString, FMError> {
    build_request_json_inner(prompt, options, None, true)
}

fn build_structured_request_json(
    prompt: &Prompt,
    options: GenerationOptions,
    schema: &GenerationSchema,
    include_schema_in_prompt: bool,
) -> Result<CString, FMError> {
    build_request_json_inner(prompt, options, Some(schema), include_schema_in_prompt)
}

fn build_request_json_inner(
    prompt: &Prompt,
    options: GenerationOptions,
    schema: Option<&GenerationSchema>,
    include_schema_in_prompt: bool,
) -> Result<CString, FMError> {
    use crate::generation::SamplingMode;
    use serde_json::json;

    let sampling = match options.sampling() {
        SamplingMode::Default => json!({ "mode": "default" }),
        SamplingMode::Greedy => json!({ "mode": "greedy" }),
        SamplingMode::TopK(k) => json!({
            "mode": "top_k",
            "topK": k,
            "seed": options.sampling_seed(),
        }),
        SamplingMode::TopP(p) => json!({
            "mode": "top_p",
            "topP": p,
            "seed": options.sampling_seed(),
        }),
    };
    let include_schema_in_prompt = schema.map_or(include_schema_in_prompt, |schema| {
        schema.effective_include_schema_in_prompt(include_schema_in_prompt)
    });
    let payload = serde_json::to_string(&json!({
        "prompt": prompt.to_bridge_value(),
        "options": {
            "temperature": options.temperature(),
            "maximumResponseTokens": options.maximum_response_tokens(),
            "sampling": sampling,
        },
        "schemaJSON": schema.map(GenerationSchema::bridge_request_json),
        "includeSchemaInPrompt": include_schema_in_prompt,
    }))
    .map_err(|e| FMError::InvalidArgument(format!("request not JSON-serializable: {e}")))?;
    CString::new(payload)
        .map_err(|e| FMError::InvalidArgument(format!("request JSON contains NUL: {e}")))
}
