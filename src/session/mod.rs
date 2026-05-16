//! [`LanguageModelSession`] — a stateful conversation with the on-device model.

use core::ffi::{c_char, c_void};
use core::ptr;
use std::ffi::CString;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use crate::error::FMError;
use crate::ffi;
use crate::generation::GenerationOptions;

/// A stateful conversation with the on-device language model.
///
/// Sessions retain their conversation history; subsequent calls to
/// [`respond`](Self::respond) build on the previous turns.
///
/// # Examples
///
/// ```rust,no_run
/// use foundation_models::LanguageModelSession;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let session = LanguageModelSession::new();
/// let answer = session.respond("Name three Norse gods.")?;
/// println!("{answer}");
/// # Ok(())
/// # }
/// ```
pub struct LanguageModelSession {
    ptr: *mut c_void,
}

// SAFETY: The underlying Swift LanguageModelSession is reference-counted via
// Unmanaged.passRetained on the Swift side; sending the opaque pointer between
// threads is safe as long as we don't dereference it from Rust (we never do —
// it only travels through extern "C" calls that internally hop to the
// Swift concurrency executor).
unsafe impl Send for LanguageModelSession {}
unsafe impl Sync for LanguageModelSession {}

impl LanguageModelSession {
    /// Create a session with the model's default behaviour.
    ///
    /// # Panics
    ///
    /// Panics if `FoundationModels` is not available on this OS. Check
    /// [`crate::SystemLanguageModel::is_available`] first if you need to
    /// handle that gracefully.
    #[must_use]
    pub fn new() -> Self {
        Self::try_new(None).expect("FoundationModels is not available on this OS")
    }

    /// Create a session with custom system instructions ("system prompt").
    ///
    /// # Panics
    ///
    /// Panics if `FoundationModels` is not available, or if `instructions`
    /// contains an interior NUL byte.
    #[must_use]
    pub fn with_instructions(instructions: &str) -> Self {
        Self::try_new(Some(instructions)).expect("FoundationModels is not available on this OS")
    }

    /// Fallible constructor. Returns `None` when `FoundationModels` is not
    /// available (OS too old, model not enabled, etc.) or when `instructions`
    /// contains an interior NUL byte.
    #[must_use]
    pub fn try_new(instructions: Option<&str>) -> Option<Self> {
        let cstring = match instructions {
            Some(s) => Some(CString::new(s).ok()?),
            None => None,
        };
        let ptr =
            unsafe { ffi::fm_session_create(cstring.as_ref().map_or(ptr::null(), |s| s.as_ptr())) };
        if ptr.is_null() {
            return None;
        }
        Some(Self { ptr })
    }

    /// Send a prompt and block until the full response is available.
    ///
    /// # Errors
    ///
    /// Returns an [`FMError`] if the model rejects the prompt, the context
    /// window is exceeded, the session is cancelled, or the prompt contains
    /// an interior NUL byte.
    pub fn respond(&self, prompt: &str) -> Result<String, FMError> {
        self.respond_with(prompt, GenerationOptions::new())
    }

    /// Pre-warm the model. Apple loads the weights + initialises the
    /// inference engine so the next `respond` call is faster. Returns
    /// immediately; the warm-up runs in the background.
    pub fn prewarm(&self) {
        unsafe { ffi::fm_session_prewarm(self.ptr) };
    }

    /// True if this session is currently producing a response (i.e. an
    /// earlier `respond` / `stream` is still in flight on Apple's queue).
    #[must_use]
    pub fn is_responding(&self) -> bool {
        unsafe { ffi::fm_session_is_responding(self.ptr) }
    }

    /// Like [`respond`](Self::respond), but with explicit generation options.
    ///
    /// # Errors
    ///
    /// See [`respond`](Self::respond).
    pub fn respond_with(
        &self,
        prompt: &str,
        options: GenerationOptions,
    ) -> Result<String, FMError> {
        let prompt_c = CString::new(prompt)
            .map_err(|e| FMError::InvalidArgument(format!("prompt contains NUL byte: {e}")))?;
        let opts = options.to_ffi();
        let (tx, rx) = mpsc::channel();
        let tx_box: Box<mpsc::Sender<Result<String, FMError>>> = Box::new(tx);
        let context = Box::into_raw(tx_box).cast::<c_void>();

        unsafe {
            ffi::fm_session_respond(
                self.ptr,
                prompt_c.as_ptr(),
                opts.temperature,
                opts.maximum_response_tokens,
                opts.sampling_mode,
                opts.top_k,
                opts.top_p,
                context,
                respond_trampoline,
            );
        }

        // The Swift side dispatches the callback on its own Task executor;
        // it is guaranteed to fire exactly once.
        rx.recv().map_err(|_| FMError::Unknown {
            code: ffi::status::UNKNOWN,
            message: "Swift bridge dropped the callback channel".into(),
        })?
    }

    /// Stream the response as the model generates it. The callback is invoked
    /// with each delta and a final invocation with `done == true`.
    ///
    /// # Errors
    ///
    /// Returns an [`FMError`] mirroring [`respond`](Self::respond). The
    /// callback may also receive a chunk *and* an error if the stream fails
    /// midway.
    pub fn stream<F>(&self, prompt: &str, mut on_chunk: F) -> Result<(), FMError>
    where
        F: FnMut(StreamEvent<'_>) + Send + 'static,
    {
        self.stream_with(prompt, GenerationOptions::new(), move |event| {
            on_chunk(event);
        })
    }

    /// Like [`stream`](Self::stream), but with explicit generation options.
    ///
    /// # Errors
    ///
    /// See [`stream`](Self::stream).
    pub fn stream_with<F>(
        &self,
        prompt: &str,
        options: GenerationOptions,
        on_chunk: F,
    ) -> Result<(), FMError>
    where
        F: FnMut(StreamEvent<'_>) + Send + 'static,
    {
        let prompt_c = CString::new(prompt)
            .map_err(|e| FMError::InvalidArgument(format!("prompt contains NUL byte: {e}")))?;
        let opts = options.to_ffi();

        // The callback may be invoked many times before completion. We pair
        // the user closure with a oneshot channel that signals "stream
        // finished" so this function can block until the Swift Task ends.
        let (done_tx, done_rx) = mpsc::channel::<Result<(), FMError>>();
        let state = Arc::new(StreamState {
            on_chunk: Mutex::new(Box::new(on_chunk)),
            done_tx: Mutex::new(Some(done_tx)),
        });
        let context = Arc::into_raw(state).cast::<c_void>().cast_mut();

        unsafe {
            ffi::fm_session_stream_response(
                self.ptr,
                prompt_c.as_ptr(),
                opts.temperature,
                opts.maximum_response_tokens,
                opts.sampling_mode,
                opts.top_k,
                opts.top_p,
                context,
                stream_trampoline,
            );
        }

        done_rx.recv().map_err(|_| FMError::Unknown {
            code: ffi::status::UNKNOWN,
            message: "Swift bridge dropped the stream channel".into(),
        })?
    }
}

impl Default for LanguageModelSession {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for LanguageModelSession {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { ffi::fm_object_release(self.ptr) };
        }
    }
}

impl core::fmt::Debug for LanguageModelSession {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("LanguageModelSession")
            .field("ptr", &self.ptr)
            .finish()
    }
}

/// One event from a streaming generation.
#[derive(Debug)]
#[non_exhaustive]
pub enum StreamEvent<'a> {
    /// Incremental text delta. Concatenate these to reconstruct the full reply.
    Chunk(&'a str),
    /// Stream finished successfully.
    Done,
    /// Stream failed; the inner error describes why.
    Error(FMError),
}

// ---------- internal callback plumbing ----------

unsafe extern "C" fn respond_trampoline(
    context: *mut c_void,
    response: *mut c_char,
    error: *mut c_char,
    status: i32,
) {
    let tx = Box::from_raw(context.cast::<mpsc::Sender<Result<String, FMError>>>());
    let result = if status == ffi::status::OK && !response.is_null() {
        let s = core::ffi::CStr::from_ptr(response)
            .to_string_lossy()
            .into_owned();
        ffi::fm_string_free(response);
        Ok(s)
    } else {
        Err(crate::error::from_swift(status, error))
    };
    let _ = tx.send(result);
}

type StreamCallback = Box<dyn FnMut(StreamEvent<'_>) + Send>;

struct StreamState {
    on_chunk: Mutex<StreamCallback>,
    done_tx: Mutex<Option<mpsc::Sender<Result<(), FMError>>>>,
}

unsafe extern "C" fn stream_trampoline(
    context: *mut c_void,
    chunk: *mut c_char,
    done: bool,
    status: i32,
) {
    let state = Arc::from_raw(context.cast::<StreamState>());
    // Bump the count back up because Swift may invoke us again before
    // `done == true` (Arc::from_raw consumed our refcount).
    let state_for_swift = state.clone();
    core::mem::forget(state_for_swift);

    let chunk_str: Option<String> = if chunk.is_null() {
        None
    } else {
        let s = core::ffi::CStr::from_ptr(chunk)
            .to_string_lossy()
            .into_owned();
        ffi::fm_string_free(chunk);
        Some(s)
    };

    if status != ffi::status::OK {
        let err = crate::error::from_swift(status, ptr::null_mut());
        let err_for_callback = chunk_str
            .map(|m| match err.clone() {
                FMError::Unknown { code, .. } => FMError::Unknown { code, message: m },
                other => other,
            })
            .unwrap_or(err);
        let mut cb = state.on_chunk.lock().expect("user callback mutex poisoned");
        cb(StreamEvent::Error(err_for_callback.clone()));
        drop(cb);
        let pending_tx = state.done_tx.lock().expect("done_tx mutex poisoned").take();
        if let Some(tx) = pending_tx {
            let _ = tx.send(Err(err_for_callback));
        }
        // This was the final invocation: drop the extra ref we forgot above.
        drop(Arc::from_raw(Arc::as_ptr(&state)));
        drop(state);
        return;
    }

    if let Some(s) = chunk_str.as_deref() {
        let mut cb = state.on_chunk.lock().expect("user callback mutex poisoned");
        cb(StreamEvent::Chunk(s));
    }

    if done {
        let mut cb = state.on_chunk.lock().expect("user callback mutex poisoned");
        cb(StreamEvent::Done);
        drop(cb);
        let pending_tx = state.done_tx.lock().expect("done_tx mutex poisoned").take();
        if let Some(tx) = pending_tx {
            let _ = tx.send(Ok(()));
        }
        // Final invocation: release the extra ref we forgot above.
        drop(Arc::from_raw(Arc::as_ptr(&state)));
    }
    drop(state);
}
