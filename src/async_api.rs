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
//! | [`AsyncAdapter::compile`] | `SystemLanguageModel.Adapter.compile()` | Returns `()` |
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

use std::ffi::{c_char, c_void, CString};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use doom_fish_utils::completion::{AsyncCompletion, AsyncCompletionFuture};
use doom_fish_utils::panic_safe::catch_user_panic_result;
use serde::Deserialize;

use crate::content::{BridgeGeneratedContent, GeneratedContent};
use crate::error::{bridge_text_result, from_swift, FMError};
use crate::ffi;
use crate::generation::GenerationOptions;
use crate::model::Adapter;
use crate::prompt::ToPrompt;
use crate::schema::GenerationSchema;
use crate::session::{decode_bridge_text_response, respond_request_json, SessionResponse};
use crate::task::SwiftTask;
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

impl OpaquePtr {
    fn into_raw(self) -> *mut c_void {
        let ptr = self.0;
        core::mem::forget(self);
        ptr
    }
}

impl Drop for OpaquePtr {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { ffi::fm_object_release(self.0) };
        }
    }
}

fn unknown_error(message: impl Into<String>) -> FMError {
    FMError::Unknown {
        code: ffi::status::UNKNOWN,
        message: message.into().into(),
    }
}

unsafe extern "C" fn text_async_cb(
    ctx: *mut c_void,
    response: *mut c_char,
    error: *mut c_char,
    status: i32,
) {
    let result = catch_user_panic_result("async response callback", || unsafe {
        bridge_text_result(response, error, status)
    })
    .unwrap_or_else(|| Err(unknown_error("async response callback panicked")));
    unsafe { AsyncCompletion::<Result<String, FMError>>::complete_ok(ctx, result) };
}

unsafe extern "C" fn object_async_cb(
    ctx: *mut c_void,
    object: *mut c_void,
    error: *mut c_char,
    status: i32,
) {
    let result = catch_user_panic_result("async object callback", || {
        let object = OpaquePtr(object);
        if status != ffi::status::OK {
            return Err(from_swift(status, error));
        }
        if !error.is_null() {
            unsafe { ffi::fm_string_free(error) };
        }
        if object.0.is_null() {
            return Err(unknown_error("Swift bridge returned a null object"));
        }
        Ok(object)
    })
    .unwrap_or_else(|| Err(unknown_error("async object callback panicked")));
    unsafe { AsyncCompletion::<Result<OpaquePtr, FMError>>::complete_ok(ctx, result) };
}

pub(crate) struct PendingText {
    inner: AsyncCompletionFuture<Result<String, FMError>>,
    _task: Option<SwiftTask>,
}

impl PendingText {
    pub(crate) fn start<F>(invoke: F) -> Self
    where
        F: FnOnce(*mut c_void, ffi::FmRespondCallback) -> *mut c_void,
    {
        let (inner, context) = AsyncCompletion::create();
        let task = SwiftTask::from_raw(invoke(context, text_async_cb));
        Self { inner, _task: task }
    }
}

impl Future for PendingText {
    type Output = Result<String, FMError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.inner)
            .poll(cx)
            .map(|result| result.unwrap_or_else(|message| Err(unknown_error(message))))
    }
}

// ============================================================================
// RespondFuture — LanguageModelSession.respond(to:)
// ============================================================================

/// Future returned by [`AsyncSession::respond`].
///
/// Resolves to `Result<SessionResponse<String>, FMError>`.
pub struct RespondFuture {
    pending: PendingText,
}

impl std::fmt::Debug for RespondFuture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RespondFuture").finish_non_exhaustive()
    }
}

impl Future for RespondFuture {
    type Output = Result<SessionResponse<String>, FMError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.pending)
            .poll(cx)
            .map(|result| result.and_then(|json| decode_bridge_text_response(&json)))
    }
}

// ============================================================================
// RespondGeneratingFuture — LanguageModelSession.respond(to:generating:)
// ============================================================================

/// Future returned by [`AsyncSession::respond_generating`].
///
/// Resolves to `Result<SessionResponse<GeneratedContent>, FMError>`.
pub struct RespondGeneratingFuture {
    pending: PendingText,
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
        Pin::new(&mut self.pending).poll(cx).map(|result| {
            let json = result?;
            let response: AsyncBridgeStructuredResponse = serde_json::from_str(&json)
                .map_err(|e| FMError::DecodingFailure(e.to_string().into()))?;
            Ok(SessionResponse {
                content: GeneratedContent::from_bridge_payload(response.content, true)?,
                raw_content: GeneratedContent::from_bridge_payload(response.raw_content, true)?,
                transcript: Transcript::from_json_str(&response.transcript_json)?,
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
    inner: AsyncCompletionFuture<Result<OpaquePtr, FMError>>,
    _task: Option<SwiftTask>,
}

impl std::fmt::Debug for AdapterInitFuture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdapterInitFuture").finish_non_exhaustive()
    }
}

impl Future for AdapterInitFuture {
    type Output = Result<Adapter, FMError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.inner).poll(cx).map(|result| {
            result
                .unwrap_or_else(|message| Err(unknown_error(message)))
                .map(|object| Adapter {
                    ptr: object.into_raw(),
                })
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
    pending: PendingText,
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
        Pin::new(&mut self.pending).poll(cx).map(|result| {
            serde_json::from_str::<Vec<String>>(&result?)
                .map_err(|e| FMError::DecodingFailure(e.to_string().into()))
        })
    }
}

/// Future returned by [`AsyncAdapter::compile`].
///
/// Resolves to `Result<(), FMError>`.
pub struct CompileAdapterFuture {
    pending: PendingText,
}

impl std::fmt::Debug for CompileAdapterFuture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompileAdapterFuture")
            .finish_non_exhaustive()
    }
}

impl Future for CompileAdapterFuture {
    type Output = Result<(), FMError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.pending)
            .poll(cx)
            .map(|result| result.map(drop))
    }
}

// ============================================================================
// AsyncSession — async wrapper around LanguageModelSession
// ============================================================================

/// Async wrapper around [`crate::LanguageModelSession`].
///
/// Futures own everything they need: dropping the session while a future is
/// pending is allowed, and dropping a future cancels its generation.
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

    fn start(&self, payload: &CString) -> PendingText {
        let session = self.session.as_ptr();
        PendingText::start(|context, callback| unsafe {
            ffi::fm_session_respond_request_json(session, payload.as_ptr(), context, callback)
        })
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
        self.respond_with_options(prompt, GenerationOptions::new())
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
        let payload = respond_request_json(&prompt.to_prompt()?, options, None, true)?;
        Ok(RespondFuture {
            pending: self.start(&payload),
        })
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
        let payload = respond_request_json(
            &prompt.to_prompt()?,
            options,
            Some(schema),
            include_schema_in_prompt,
        )?;
        Ok(RespondGeneratingFuture {
            pending: self.start(&payload),
        })
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
    /// Returns the SDK's typed [`FMError`] (for example
    /// [`FMError::AdapterInvalidName`]) if the adapter can't be loaded, or
    /// [`FMError::InvalidArgument`] if the name contains a NUL byte.
    pub fn from_name(name: &str) -> Result<AdapterInitFuture, FMError> {
        let cname = CString::new(name).map_err(|e| {
            FMError::InvalidArgument(format!("NUL byte in adapter name: {e}").into())
        })?;
        let (inner, context) = AsyncCompletion::create();
        let task = SwiftTask::from_raw(unsafe {
            ffi::fm_adapter_create_from_name_async(cname.as_ptr(), context, object_async_cb)
        });
        Ok(AdapterInitFuture { inner, _task: task })
    }

    /// Async version of `SystemLanguageModel.Adapter.compatibility(for:)`.
    ///
    /// Returns the list of compatible adapter identifiers for the given
    /// logical adapter name.
    ///
    /// # Errors
    ///
    /// Returns an [`FMError`] if the name contains a NUL byte or the bridge
    /// fails.
    pub fn compatibility(name: &str) -> Result<AdapterCompatibilityFuture, FMError> {
        let cname = CString::new(name).map_err(|e| {
            FMError::InvalidArgument(format!("NUL byte in adapter name: {e}").into())
        })?;
        Ok(AdapterCompatibilityFuture {
            pending: PendingText::start(|context, callback| unsafe {
                ffi::fm_adapter_compatibility_async(cname.as_ptr(), context, callback)
            }),
        })
    }

    /// Async version of `SystemLanguageModel.Adapter.compile()`.
    #[must_use]
    pub fn compile(adapter: &Adapter) -> CompileAdapterFuture {
        let adapter = adapter.ptr;
        CompileAdapterFuture {
            pending: PendingText::start(|context, callback| unsafe {
                ffi::fm_adapter_compile(adapter, context, callback)
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn bridge_string(value: &str) -> *mut c_char {
        let value = CString::new(value).unwrap();
        unsafe { ffi::fm_string_dup(value.as_ptr()) }
    }

    fn complete_text(
        response: Option<&str>,
        error: Option<&str>,
        status: i32,
    ) -> Result<String, FMError> {
        let pending = PendingText::start(|context, callback| {
            unsafe {
                callback(
                    context,
                    response.map_or(core::ptr::null_mut(), bridge_string),
                    error.map_or(core::ptr::null_mut(), bridge_string),
                    status,
                );
            }
            core::ptr::null_mut()
        });
        pollster::block_on(pending)
    }

    #[test]
    fn async_errors_keep_their_type_and_metadata() {
        let error = complete_text(
            None,
            Some(
                &json!({
                    "message": "request refused",
                    "recoverySuggestion": "Try a safer prompt",
                    "refusal": { "token": "refusal-token" }
                })
                .to_string(),
            ),
            ffi::status::REFUSAL,
        )
        .expect_err("refusal must fail");
        assert!(matches!(error, FMError::Refusal(ref message) if message == "request refused"));
        assert_eq!(
            error.recovery_suggestion().as_deref(),
            Some("Try a safer prompt")
        );
        assert!(error.refusal().is_some());

        let error = complete_text(None, Some("busy"), ffi::status::CONCURRENT_REQUESTS)
            .expect_err("busy session must fail");
        assert!(matches!(error, FMError::ConcurrentRequests(_)));

        let error = complete_text(None, None, ffi::status::CANCELLED).expect_err("cancelled");
        assert_eq!(error, FMError::Cancelled);
    }

    #[test]
    fn async_success_returns_the_response_text() {
        assert_eq!(
            complete_text(Some("payload"), None, ffi::status::OK).as_deref(),
            Ok("payload")
        );
        assert!(complete_text(None, None, ffi::status::OK).is_err());
    }

    #[test]
    fn async_object_errors_are_typed() {
        let (inner, context) = AsyncCompletion::<Result<OpaquePtr, FMError>>::create();
        unsafe {
            object_async_cb(
                context,
                core::ptr::null_mut(),
                bridge_string("adapter missing"),
                ffi::status::ADAPTER_INVALID_NAME,
            );
        }
        let future = AdapterInitFuture { inner, _task: None };
        let error = pollster::block_on(future).expect_err("adapter lookup must fail");
        assert!(
            matches!(error, FMError::AdapterInvalidName(ref message) if message == "adapter missing")
        );
    }
}
