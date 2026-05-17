//! [`LanguageModelSession`] — a stateful conversation with the on-device model.

use core::ffi::{c_char, c_void};
use core::ptr;
use std::ffi::CString;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use serde::Deserialize;
use serde_json::json;

use crate::content::{BridgeGeneratedContent, GeneratedContent};
use crate::error::FMError;
use crate::ffi;
use crate::generation::{GenerationOptions, SamplingMode};
use crate::model::ConfiguredSystemLanguageModel;
use crate::prompt::{Instructions, Prompt, ToInstructions, ToPrompt};
use crate::schema::GenerationSchema;
use crate::tool::{tool_callback_trampoline, Tool, ToolRegistry};
use crate::transcript::Transcript;

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
    _tool_registry: Option<Arc<ToolRegistry>>,
}

// SAFETY: The underlying Swift LanguageModelSession is reference-counted via
// Unmanaged.passRetained on the Swift side; sending the opaque pointer between
// threads is safe as long as we don't dereference it from Rust (we never do —
// it only travels through extern "C" calls that internally hop to the
// Swift concurrency executor).
unsafe impl Send for LanguageModelSession {}
unsafe impl Sync for LanguageModelSession {}

impl LanguageModelSession {
    /// Return the raw opaque pointer to the underlying Swift session object.
    ///
    /// Used internally by `async_api` to pass the session pointer to FFI
    /// callbacks without exposing `ptr` as a public field.
    pub(crate) fn as_ptr(&self) -> *mut c_void {
        self.ptr
    }

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
        Some(Self {
            ptr,
            _tool_registry: None,
        })
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

    /// Return a best-effort JSON serialisation of the session's
    /// `Transcript` — the full history of user prompts and model
    /// responses. Useful for persisting a chat session across
    /// process boundaries.
    #[must_use]
    pub fn transcript_json(&self) -> String {
        let p = unsafe { ffi::fm_session_transcript_json(self.ptr) };
        if p.is_null() {
            return String::from("{}");
        }
        let s = unsafe { core::ffi::CStr::from_ptr(p) }
            .to_string_lossy()
            .into_owned();
        unsafe { ffi::fm_string_free(p) };
        s
    }

    /// Log feedback on the most recent response for diagnostic /
    /// fine-tuning purposes. `sentiment`:
    /// `1` positive, `0` neutral, `-1` negative.
    pub fn log_feedback(&self, sentiment: i32, description: Option<&str>) {
        let cstr = description.and_then(|s| CString::new(s).ok());
        let p = cstr.as_ref().map_or(core::ptr::null(), |c| c.as_ptr());
        unsafe { ffi::fm_session_log_feedback(self.ptr, sentiment, p) };
    }

    /// Prompt-engineered JSON-shape response.
    ///
    /// Wraps the prompt with a "respond with valid JSON matching this schema"
    /// instruction and parses the response. The schema is a
    /// `serde_json::Value`-style JSON string (passed as text).
    ///
    /// Useful for getting structured data out of the model without the
    /// full Generable macro machinery. The model still returns plain
    /// text — the caller must parse with `serde_json` / `serde` after.
    ///
    /// # Errors
    ///
    /// See [`respond`](Self::respond).
    pub fn respond_with_json_schema(
        &self,
        prompt: &str,
        schema_description: &str,
    ) -> Result<String, FMError> {
        let wrapped = format!(
            "{prompt}\n\n\
             IMPORTANT: respond with VALID JSON ONLY (no prose, no markdown \
             fences) that matches this schema:\n\n{schema_description}\n\n\
             Your entire response must be parseable by JSON.parse()."
        );
        self.respond(&wrapped)
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
        self.respond_prompt_with(prompt, options)
    }

    /// Schema-driven structured response.
    ///
    /// Builds a `DynamicGenerationSchema` from the provided JSON
    /// schema, runs `LanguageModelSession.respond(schema:prompt:)`,
    /// and returns the model's `GeneratedContent.jsonString` — a
    /// well-formed JSON string matching the requested shape.
    ///
    /// Supported `schema` shape (strict subset of JSON Schema):
    ///
    /// ```json
    /// {
    ///   "type": "object",
    ///   "name": "Movie",
    ///   "properties": {
    ///     "title":  { "type": "string", "description": "Movie title" },
    ///     "year":   { "type": "integer" },
    ///     "rating": { "type": "number", "optional": true },
    ///     "tags":   { "type": "array", "items": { "type": "string" }, "min": 1, "max": 5 }
    ///   }
    /// }
    /// ```
    ///
    /// Primitive types: `"string"`, `"integer"`, `"number"`,
    /// `"boolean"`, `"array"`, `"object"`. Each property may set
    /// `"description"` and `"optional"`. Array schemas accept
    /// `"items"` plus optional `"min"` / `"max"` element counts.
    ///
    /// # Errors
    ///
    /// See [`respond`](Self::respond) for general errors, plus a
    /// "schema build failed" / "schema JSON is not valid" error
    /// returned as [`FMError::Unknown`] if the schema is malformed.
    pub fn respond_with_schema(
        &self,
        prompt: &str,
        schema: &str,
        include_schema_in_prompt: bool,
    ) -> Result<String, FMError> {
        self.respond_with_schema_options(
            prompt,
            schema,
            include_schema_in_prompt,
            GenerationOptions::new(),
        )
    }

    /// [`respond_with_schema`](Self::respond_with_schema) with
    /// explicit generation options.
    ///
    /// # Errors
    ///
    /// See [`respond_with_schema`](Self::respond_with_schema).
    pub fn respond_with_schema_options(
        &self,
        prompt: &str,
        schema: &str,
        include_schema_in_prompt: bool,
        options: GenerationOptions,
    ) -> Result<String, FMError> {
        let prompt_c = CString::new(prompt)
            .map_err(|e| FMError::InvalidArgument(format!("prompt NUL byte: {e}")))?;
        let schema_c = CString::new(schema)
            .map_err(|e| FMError::InvalidArgument(format!("schema NUL byte: {e}")))?;
        let opts = options.to_ffi();
        let (tx, rx) = mpsc::channel();
        let tx_box: Box<mpsc::Sender<Result<String, FMError>>> = Box::new(tx);
        let context = Box::into_raw(tx_box).cast::<c_void>();

        unsafe {
            ffi::fm_session_respond_with_schema(
                self.ptr,
                prompt_c.as_ptr(),
                schema_c.as_ptr(),
                include_schema_in_prompt,
                opts.temperature,
                opts.maximum_response_tokens,
                opts.sampling_mode,
                opts.top_k,
                opts.top_p,
                context,
                respond_trampoline,
            );
        }

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
        let payload = respond_request_json(&Prompt::from(prompt), options, None, true)?;

        let (done_tx, done_rx) = mpsc::channel::<Result<(), FMError>>();
        let state = Arc::new(StreamState {
            on_chunk: Mutex::new(Box::new(on_chunk)),
            done_tx: Mutex::new(Some(done_tx)),
        });
        let context = Arc::into_raw(state).cast::<c_void>().cast_mut();

        unsafe {
            ffi::fm_session_stream_request_json(
                self.ptr,
                payload.as_ptr(),
                context,
                json_text_stream_trampoline,
            )
        };

        done_rx.recv().map_err(|_| FMError::Unknown {
            code: ffi::status::UNKNOWN,
            message: "Swift bridge dropped the stream channel".into(),
        })?
    }
}

impl LanguageModelSession {
    /// Create a configurable session builder.
    #[must_use]
    pub fn builder<'a>() -> SessionBuilder<'a> {
        SessionBuilder::new()
    }

    /// Restore a session from a transcript.
    ///
    /// # Errors
    ///
    /// Returns an [`FMError`] if the transcript cannot be encoded for Swift.
    pub fn from_transcript(transcript: Transcript) -> Result<Self, FMError> {
        Self::builder().transcript(transcript).build()
    }

    /// Return the typed transcript for this session.
    ///
    /// # Errors
    ///
    /// Returns an [`FMError`] if the transcript JSON returned by Swift could not
    /// be decoded.
    pub fn transcript(&self) -> Result<Transcript, FMError> {
        Transcript::from_json_str(&self.transcript_json())
    }

    /// Pre-warm the model using a prompt prefix.
    ///
    /// # Errors
    ///
    /// Returns an [`FMError`] if the prompt cannot be encoded for Swift.
    pub fn prewarm_with_prompt<P>(&self, prompt: P) -> Result<(), FMError>
    where
        P: ToPrompt,
    {
        let prompt = prompt.to_prompt()?;
        let prompt_json = CString::new(prompt.to_bridge_json()?).map_err(|error| {
            FMError::InvalidArgument(format!("prompt JSON contains a NUL byte: {error}"))
        })?;
        let mut error: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::fm_session_prewarm_prompt_json(self.ptr, prompt_json.as_ptr(), &mut error)
        };
        if status != ffi::status::OK {
            return Err(crate::error::from_swift(status, error));
        }
        Ok(())
    }

    /// Respond to a structured prompt and return only the generated text.
    ///
    /// # Errors
    ///
    /// Returns an [`FMError`] if generation fails.
    pub fn respond_prompt<P>(&self, prompt: P) -> Result<String, FMError>
    where
        P: ToPrompt,
    {
        self.respond_prompt_with(prompt, GenerationOptions::new())
    }

    /// Like [`respond_prompt`](Self::respond_prompt), but with explicit options.
    ///
    /// # Errors
    ///
    /// Returns an [`FMError`] if generation fails.
    pub fn respond_prompt_with<P>(
        &self,
        prompt: P,
        options: GenerationOptions,
    ) -> Result<String, FMError>
    where
        P: ToPrompt,
    {
        self.respond_prompt_detailed(prompt, options)
            .map(|response| response.content)
    }

    /// Respond to a structured prompt and keep the full response metadata.
    ///
    /// # Errors
    ///
    /// Returns an [`FMError`] if generation fails.
    pub fn respond_prompt_detailed<P>(
        &self,
        prompt: P,
        options: GenerationOptions,
    ) -> Result<SessionResponse<String>, FMError>
    where
        P: ToPrompt,
    {
        let prompt = prompt.to_prompt()?;
        let payload = respond_request_json(&prompt, options, None, true)?;
        let payload = request_response(self.ptr, &payload)?;
        let response: BridgeTextResponse = serde_json::from_str(&payload)
            .map_err(|error| FMError::DecodingFailure(error.to_string()))?;
        Ok(SessionResponse {
            content: response.content,
            raw_content: GeneratedContent::from_bridge_payload(response.raw_content, true)?,
            transcript: Transcript::from_json_str(&response.transcript_json)?,
        })
    }

    /// Generate structured content using an explicit schema.
    ///
    /// # Errors
    ///
    /// Returns an [`FMError`] if generation fails or the schema is invalid.
    pub fn respond_generated<P>(
        &self,
        prompt: P,
        schema: &GenerationSchema,
        include_schema_in_prompt: bool,
    ) -> Result<GeneratedContent, FMError>
    where
        P: ToPrompt,
    {
        self.respond_generated_with(
            prompt,
            schema,
            include_schema_in_prompt,
            GenerationOptions::new(),
        )
        .map(|response| response.content)
    }

    /// Like [`respond_generated`](Self::respond_generated), but with explicit options.
    ///
    /// # Errors
    ///
    /// Returns an [`FMError`] if generation fails or the schema is invalid.
    pub fn respond_generated_with<P>(
        &self,
        prompt: P,
        schema: &GenerationSchema,
        include_schema_in_prompt: bool,
        options: GenerationOptions,
    ) -> Result<SessionResponse<GeneratedContent>, FMError>
    where
        P: ToPrompt,
    {
        let prompt = prompt.to_prompt()?;
        let payload =
            respond_request_json(&prompt, options, Some(schema), include_schema_in_prompt)?;
        let payload = request_response(self.ptr, &payload)?;
        let response: BridgeStructuredResponse = serde_json::from_str(&payload)
            .map_err(|error| FMError::DecodingFailure(error.to_string()))?;
        Ok(SessionResponse {
            content: GeneratedContent::from_bridge_payload(response.content, true)?,
            raw_content: GeneratedContent::from_bridge_payload(response.raw_content, true)?,
            transcript: Transcript::from_json_str(&response.transcript_json)?,
        })
    }

    /// Generate a typed Rust value using a [`crate::schema::Generable`] implementation.
    ///
    /// # Errors
    ///
    /// Returns an [`FMError`] if generation fails or the generated JSON cannot
    /// be decoded as `T`.
    pub fn respond_generating<P, T>(
        &self,
        prompt: P,
        include_schema_in_prompt: bool,
        options: GenerationOptions,
    ) -> Result<SessionResponse<T>, FMError>
    where
        P: ToPrompt,
        T: crate::schema::Generable,
    {
        let response = self.respond_generated_with(
            prompt,
            &T::generation_schema()?,
            include_schema_in_prompt,
            options,
        )?;
        Ok(SessionResponse {
            content: T::from_generated_content(&response.content)?,
            raw_content: response.raw_content,
            transcript: response.transcript,
        })
    }

    /// Stream a structured prompt token-by-token.
    ///
    /// # Errors
    ///
    /// Returns an [`FMError`] if the prompt cannot be encoded or generation fails.
    pub fn stream_prompt<P, F>(&self, prompt: P, on_chunk: F) -> Result<(), FMError>
    where
        P: ToPrompt,
        F: FnMut(StreamEvent<'_>) + Send + 'static,
    {
        let prompt = prompt.to_prompt()?;
        let prompt_text = prompt_to_plain_text(&prompt).ok_or_else(|| {
            FMError::InvalidArgument(
                "text streaming only supports prompts composed of text segments".into(),
            )
        })?;
        self.stream_with(&prompt_text, GenerationOptions::new(), on_chunk)
    }

    /// Stream structured generation snapshots.
    ///
    /// # Errors
    ///
    /// Returns an [`FMError`] if the prompt cannot be encoded or generation fails.
    pub fn stream_generated<P, F>(
        &self,
        prompt: P,
        schema: &GenerationSchema,
        include_schema_in_prompt: bool,
        options: GenerationOptions,
        on_event: F,
    ) -> Result<(), FMError>
    where
        P: ToPrompt,
        F: FnMut(StructuredStreamEvent) + Send + 'static,
    {
        let prompt = prompt.to_prompt()?;
        let payload =
            respond_request_json(&prompt, options, Some(schema), include_schema_in_prompt)?;
        let (done_tx, done_rx) = mpsc::channel::<Result<(), FMError>>();
        let state = Arc::new(StructuredStreamState {
            on_event: Mutex::new(Box::new(on_event)),
            done_tx: Mutex::new(Some(done_tx)),
        });
        let context = Arc::into_raw(state).cast::<c_void>().cast_mut();
        unsafe {
            ffi::fm_session_stream_request_json(
                self.ptr,
                payload.as_ptr(),
                context,
                structured_stream_trampoline,
            )
        };
        done_rx.recv().map_err(|_| FMError::Unknown {
            code: ffi::status::UNKNOWN,
            message: "Swift bridge dropped the structured stream channel".into(),
        })?
    }

    /// Log a feedback attachment and return the raw bytes Apple produced.
    ///
    /// # Errors
    ///
    /// Returns an [`FMError`] if the attachment request is invalid.
    pub fn log_feedback_attachment(
        &self,
        request: FeedbackAttachmentRequest,
    ) -> Result<Vec<u8>, FMError> {
        let request_json = CString::new(request.to_bridge_json()?).map_err(|error| {
            FMError::InvalidArgument(format!("feedback request contains a NUL byte: {error}"))
        })?;
        let mut length = 0usize;
        let mut error: *mut c_char = ptr::null_mut();
        let ptr = unsafe {
            ffi::fm_session_log_feedback_attachment_json(
                self.ptr,
                request_json.as_ptr(),
                &mut length,
                &mut error,
            )
        };
        if ptr.is_null() && !error.is_null() {
            return Err(crate::error::from_swift(
                ffi::status::INVALID_ARGUMENT,
                error,
            ));
        }
        if ptr.is_null() || length == 0 {
            return Ok(Vec::new());
        }
        let bytes = unsafe { std::slice::from_raw_parts(ptr.cast::<u8>(), length) }.to_vec();
        unsafe { ffi::fm_bytes_free(ptr) };
        Ok(bytes)
    }
}

/// Builder for [`LanguageModelSession`].
pub struct SessionBuilder<'a> {
    model: Option<&'a ConfiguredSystemLanguageModel>,
    instructions: Option<Instructions>,
    transcript: Option<Transcript>,
    tools: Vec<Tool>,
}

impl<'a> SessionBuilder<'a> {
    const fn new() -> Self {
        Self {
            model: None,
            instructions: None,
            transcript: None,
            tools: Vec::new(),
        }
    }

    /// Use a configured system model.
    #[must_use]
    pub const fn model(mut self, model: &'a ConfiguredSystemLanguageModel) -> Self {
        self.model = Some(model);
        self
    }

    /// Set system instructions.
    pub fn instructions<I>(mut self, instructions: I) -> Result<Self, FMError>
    where
        I: ToInstructions,
    {
        self.instructions = Some(instructions.to_instructions()?);
        Ok(self)
    }

    /// Restore the session from a transcript.
    #[must_use]
    pub fn transcript(mut self, transcript: Transcript) -> Self {
        self.transcript = Some(transcript);
        self
    }

    /// Add one tool.
    #[must_use]
    pub fn tool(mut self, tool: Tool) -> Self {
        self.tools.push(tool);
        self
    }

    /// Add many tools.
    #[must_use]
    pub fn tools(mut self, tools: impl IntoIterator<Item = Tool>) -> Self {
        self.tools.extend(tools);
        self
    }

    /// Build the session.
    ///
    /// # Errors
    ///
    /// Returns an [`FMError`] if the configuration cannot be encoded for Swift.
    pub fn build(self) -> Result<LanguageModelSession, FMError> {
        if self.instructions.is_some() && self.transcript.is_some() {
            return Err(FMError::InvalidArgument(
                "session builder accepts either instructions or a transcript, not both".into(),
            ));
        }

        let instructions_json = self
            .instructions
            .as_ref()
            .map(Instructions::to_bridge_json)
            .transpose()?;
        let transcript_json = self
            .transcript
            .as_ref()
            .map(Transcript::to_json_string)
            .transpose()?;
        let tool_registry = if self.tools.is_empty() {
            None
        } else {
            Some(Arc::new(ToolRegistry::new(self.tools)))
        };
        let tools_json = tool_registry
            .as_ref()
            .map(|registry| registry.specs_json())
            .transpose()?;

        let instructions_c = instructions_json
            .as_deref()
            .map(CString::new)
            .transpose()
            .map_err(|error| {
                FMError::InvalidArgument(format!("instructions JSON contains a NUL byte: {error}"))
            })?;
        let transcript_c = transcript_json
            .as_deref()
            .map(CString::new)
            .transpose()
            .map_err(|error| {
                FMError::InvalidArgument(format!("transcript JSON contains a NUL byte: {error}"))
            })?;
        let tools_c = tools_json
            .as_deref()
            .map(CString::new)
            .transpose()
            .map_err(|error| {
                FMError::InvalidArgument(format!("tool JSON contains a NUL byte: {error}"))
            })?;

        let tool_context = tool_registry.as_ref().map_or(ptr::null_mut(), |registry| {
            Arc::as_ptr(registry).cast_mut().cast::<c_void>()
        });
        let mut error: *mut c_char = ptr::null_mut();
        let ptr = unsafe {
            ffi::fm_session_create_ex(
                self.model.map_or(ptr::null_mut(), |model| model.ptr),
                instructions_c
                    .as_ref()
                    .map_or(ptr::null(), |json| json.as_ptr()),
                transcript_c
                    .as_ref()
                    .map_or(ptr::null(), |json| json.as_ptr()),
                tools_c.as_ref().map_or(ptr::null(), |json| json.as_ptr()),
                tool_context,
                tool_registry
                    .as_ref()
                    .map(|_| tool_callback_trampoline as ffi::FmToolCallback),
                &mut error,
            )
        };
        if ptr.is_null() {
            return Err(crate::error::from_swift(
                ffi::status::MODEL_UNAVAILABLE,
                error,
            ));
        }
        Ok(LanguageModelSession {
            ptr,
            _tool_registry: tool_registry,
        })
    }
}

/// A detailed generation response.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionResponse<T> {
    pub content: T,
    pub raw_content: GeneratedContent,
    pub transcript: Transcript,
}

/// One structured-generation stream snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuredStreamSnapshot {
    pub content_json: String,
    pub raw_content_json: String,
    pub is_complete: bool,
}

/// One structured stream event.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum StructuredStreamEvent {
    Snapshot(StructuredStreamSnapshot),
    Done,
    Error(FMError),
}

/// One feedback issue category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedbackIssueCategory {
    Unhelpful,
    TooVerbose,
    DidNotFollowInstructions,
    Incorrect,
    StereotypeOrBias,
    SuggestiveOrSexual,
    VulgarOrOffensive,
    TriggeredGuardrailUnexpectedly,
}

impl FeedbackIssueCategory {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Unhelpful => "unhelpful",
            Self::TooVerbose => "too_verbose",
            Self::DidNotFollowInstructions => "did_not_follow_instructions",
            Self::Incorrect => "incorrect",
            Self::StereotypeOrBias => "stereotype_or_bias",
            Self::SuggestiveOrSexual => "suggestive_or_sexual",
            Self::VulgarOrOffensive => "vulgar_or_offensive",
            Self::TriggeredGuardrailUnexpectedly => "triggered_guardrail_unexpectedly",
        }
    }
}

/// One feedback issue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedbackIssue {
    pub category: FeedbackIssueCategory,
    pub explanation: Option<String>,
}

/// Feedback sentiment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedbackSentiment {
    Positive,
    Negative,
    Neutral,
}

impl FeedbackSentiment {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Positive => "positive",
            Self::Negative => "negative",
            Self::Neutral => "neutral",
        }
    }
}

/// A full feedback attachment request.
#[derive(Debug, Clone, PartialEq)]
pub struct FeedbackAttachmentRequest {
    pub sentiment: Option<FeedbackSentiment>,
    pub issues: Vec<FeedbackIssue>,
    pub desired_response_text: Option<String>,
    pub desired_response_content: Option<GeneratedContent>,
    pub desired_output: Option<crate::transcript::Entry>,
}

impl FeedbackAttachmentRequest {
    /// Create an empty feedback request.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            sentiment: None,
            issues: Vec::new(),
            desired_response_text: None,
            desired_response_content: None,
            desired_output: None,
        }
    }

    fn to_bridge_json(&self) -> Result<String, FMError> {
        let issues = self
            .issues
            .iter()
            .map(|issue| {
                json!({
                    "category": issue.category.as_str(),
                    "explanation": issue.explanation,
                })
            })
            .collect::<Vec<_>>();
        let desired_output_json = self
            .desired_output
            .as_ref()
            .map(|entry| Transcript::from(vec![entry.clone()]).to_json_string())
            .transpose()?;
        let desired_response_content = self
            .desired_response_content
            .as_ref()
            .map(GeneratedContent::to_bridge_value)
            .transpose()?;
        serde_json::to_string(&json!({
            "sentiment": self.sentiment.map(FeedbackSentiment::as_str),
            "issues": issues,
            "desiredResponseText": self.desired_response_text,
            "desiredResponseContent": desired_response_content,
            "desiredOutputTranscriptJSON": desired_output_json,
        }))
        .map_err(|error| {
            FMError::InvalidArgument(format!(
                "feedback request is not JSON-serializable: {error}"
            ))
        })
    }
}

#[derive(Debug, Deserialize)]
struct BridgeTextResponse {
    content: String,
    #[serde(rename = "rawContent")]
    raw_content: BridgeGeneratedContent,
    #[serde(rename = "transcriptJSON")]
    transcript_json: String,
}

#[derive(Debug, Deserialize)]
struct BridgeStructuredResponse {
    content: BridgeGeneratedContent,
    #[serde(rename = "rawContent")]
    raw_content: BridgeGeneratedContent,
    #[serde(rename = "transcriptJSON")]
    transcript_json: String,
}

#[derive(Debug, Deserialize)]
struct BridgeStructuredSnapshot {
    content: BridgeGeneratedContent,
    #[serde(rename = "rawContent")]
    raw_content: BridgeGeneratedContent,
    #[serde(rename = "isComplete")]
    is_complete: bool,
}

#[derive(Debug, Deserialize)]
struct BridgeTextStreamSnapshot {
    delta: String,
}

fn respond_request_json(
    prompt: &Prompt,
    options: GenerationOptions,
    schema: Option<&GenerationSchema>,
    include_schema_in_prompt: bool,
) -> Result<CString, FMError> {
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
    let payload = serde_json::to_string(&json!({
        "prompt": prompt.to_bridge_value(),
        "options": {
            "temperature": options.temperature(),
            "maximumResponseTokens": options.maximum_response_tokens(),
            "sampling": sampling,
        },
        "schemaJSON": schema.map(GenerationSchema::json_schema),
        "includeSchemaInPrompt": include_schema_in_prompt,
    }))
    .map_err(|error| {
        FMError::InvalidArgument(format!("request is not JSON-serializable: {error}"))
    })?;
    CString::new(payload).map_err(|error| {
        FMError::InvalidArgument(format!("request JSON contains a NUL byte: {error}"))
    })
}

fn request_response(session: *mut c_void, payload: &CString) -> Result<String, FMError> {
    let (tx, rx) = mpsc::channel();
    let tx_box: Box<mpsc::Sender<Result<String, FMError>>> = Box::new(tx);
    let context = Box::into_raw(tx_box).cast::<c_void>();
    unsafe {
        ffi::fm_session_respond_request_json(session, payload.as_ptr(), context, respond_trampoline)
    };
    rx.recv().map_err(|_| FMError::Unknown {
        code: ffi::status::UNKNOWN,
        message: "Swift bridge dropped the JSON response channel".into(),
    })?
}

pub(crate) fn decode_bridge_text_response(
    payload: &str,
) -> Result<SessionResponse<String>, FMError> {
    let response: BridgeTextResponse = serde_json::from_str(payload)
        .map_err(|error| FMError::DecodingFailure(error.to_string()))?;
    Ok(SessionResponse {
        content: response.content,
        raw_content: GeneratedContent::from_bridge_payload(response.raw_content, true)?,
        transcript: Transcript::from_json_str(&response.transcript_json)?,
    })
}

pub(crate) fn request_text_response_with<F>(invoke: F) -> Result<SessionResponse<String>, FMError>
where
    F: FnOnce(*mut c_void, ffi::FmRespondCallback),
{
    let (tx, rx) = mpsc::channel();
    let tx_box: Box<mpsc::Sender<Result<String, FMError>>> = Box::new(tx);
    let context = Box::into_raw(tx_box).cast::<c_void>();
    invoke(context, respond_trampoline);
    let payload = rx.recv().map_err(|_| FMError::Unknown {
        code: ffi::status::UNKNOWN,
        message: "Swift bridge dropped the JSON response channel".into(),
    })??;
    decode_bridge_text_response(&payload)
}

pub(crate) fn run_text_stream_with<F, C>(invoke: F, on_chunk: C) -> Result<(), FMError>
where
    F: FnOnce(*mut c_void, ffi::FmStreamCallback),
    C: FnMut(StreamEvent<'_>) + Send + 'static,
{
    let (done_tx, done_rx) = mpsc::channel::<Result<(), FMError>>();
    let state = Arc::new(StreamState {
        on_chunk: Mutex::new(Box::new(on_chunk)),
        done_tx: Mutex::new(Some(done_tx)),
    });
    let context = Arc::into_raw(state).cast::<c_void>().cast_mut();
    invoke(context, json_text_stream_trampoline);
    done_rx.recv().map_err(|_| FMError::Unknown {
        code: ffi::status::UNKNOWN,
        message: "Swift bridge dropped the stream channel".into(),
    })?
}

fn prompt_to_plain_text(prompt: &Prompt) -> Option<String> {
    let mut text = String::new();
    for segment in prompt.segments() {
        match segment {
            crate::prompt::Segment::Text(segment) => text.push_str(&segment.text),
            crate::prompt::Segment::Structure(_) => return None,
        }
    }
    Some(text)
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

// SAFETY: `context` is a `Box<mpsc::Sender<...>>` raw pointer created by
// `request_response` / `request_text_response_with`. Swift calls this callback
// exactly once, so there is no double-free risk. `response` and `error` are
// C strings owned by the Swift bridge and only valid for this call.
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

// SAFETY: `context` is a `Arc<StreamState>` raw pointer passed via
// `Arc::into_raw`. We reconstruct it with `Arc::from_raw` on every call and
// immediately `mem::forget` a clone so the count stays ≥ 1 until the
// terminal call (done=true or error). `chunk` is a Swift-owned C string valid
// only for the duration of this call.
unsafe extern "C" fn json_text_stream_trampoline(
    context: *mut c_void,
    chunk: *mut c_char,
    done: bool,
    status: i32,
) {
    let state = Arc::from_raw(context.cast::<StreamState>());
    let state_for_swift = state.clone();
    core::mem::forget(state_for_swift);

    let payload: Option<String> = if chunk.is_null() {
        None
    } else {
        let value = core::ffi::CStr::from_ptr(chunk)
            .to_string_lossy()
            .into_owned();
        ffi::fm_string_free(chunk);
        Some(value)
    };

    if status != ffi::status::OK {
        let err = payload
            .map(|message| {
                crate::error::from_swift(
                    status,
                    ffi::fm_string_dup(
                        CString::new(message)
                            .expect("stream errors must not contain NUL bytes")
                            .as_ptr(),
                    ),
                )
            })
            .unwrap_or_else(|| crate::error::from_swift(status, ptr::null_mut()));
        {
            let mut cb = state.on_chunk.lock().expect("user callback mutex poisoned");
            // Catch panics so they don't unwind across the FFI boundary (UB).
            let _ = catch_unwind(AssertUnwindSafe(|| cb(StreamEvent::Error(err.clone()))));
        }
        if let Some(tx) = state.done_tx.lock().expect("done_tx mutex poisoned").take() {
            let _ = tx.send(Err(err));
        }
        drop(Arc::from_raw(Arc::as_ptr(&state)));
        drop(state);
        return;
    }

    if let Some(payload) = payload {
        match serde_json::from_str::<BridgeTextStreamSnapshot>(&payload) {
            Ok(snapshot) if !snapshot.delta.is_empty() => {
                let chunk_panicked = {
                    let mut cb = state.on_chunk.lock().expect("user callback mutex poisoned");
                    // Catch panics so they don't unwind across the FFI boundary.
                    catch_unwind(AssertUnwindSafe(|| cb(StreamEvent::Chunk(&snapshot.delta))))
                        .is_err()
                };
                if chunk_panicked {
                    if let Some(tx) =
                        state.done_tx.lock().expect("done_tx mutex poisoned").take()
                    {
                        let _ = tx.send(Err(FMError::Unknown {
                            code: ffi::status::UNKNOWN,
                            message: "stream callback panicked".into(),
                        }));
                    }
                    drop(Arc::from_raw(Arc::as_ptr(&state)));
                    drop(state);
                    return;
                }
            }
            Ok(_) => {}
            Err(error) => {
                let err = FMError::DecodingFailure(error.to_string());
                {
                    let mut cb = state.on_chunk.lock().expect("user callback mutex poisoned");
                    let _ = catch_unwind(AssertUnwindSafe(|| cb(StreamEvent::Error(err.clone()))));
                }
                if let Some(tx) = state.done_tx.lock().expect("done_tx mutex poisoned").take() {
                    let _ = tx.send(Err(err));
                }
                drop(Arc::from_raw(Arc::as_ptr(&state)));
                drop(state);
                return;
            }
        }
    }

    if done {
        {
            let mut cb = state.on_chunk.lock().expect("user callback mutex poisoned");
            let _ = catch_unwind(AssertUnwindSafe(|| cb(StreamEvent::Done)));
        }
        if let Some(tx) = state.done_tx.lock().expect("done_tx mutex poisoned").take() {
            let _ = tx.send(Ok(()));
        }
        drop(Arc::from_raw(Arc::as_ptr(&state)));
    }
    drop(state);
}

type StructuredStreamCallback = Box<dyn FnMut(StructuredStreamEvent) + Send>;

struct StructuredStreamState {
    on_event: Mutex<StructuredStreamCallback>,
    done_tx: Mutex<Option<mpsc::Sender<Result<(), FMError>>>>,
}

// SAFETY: Same invariants as `json_text_stream_trampoline` above, but for
// `StructuredStreamState`.
#[allow(clippy::too_many_lines)]
unsafe extern "C" fn structured_stream_trampoline(
    context: *mut c_void,
    chunk: *mut c_char,
    done: bool,
    status: i32,
) {
    let state = Arc::from_raw(context.cast::<StructuredStreamState>());
    let state_for_swift = state.clone();
    core::mem::forget(state_for_swift);

    let payload: Option<String> = if chunk.is_null() {
        None
    } else {
        let value = core::ffi::CStr::from_ptr(chunk)
            .to_string_lossy()
            .into_owned();
        ffi::fm_string_free(chunk);
        Some(value)
    };

    if status != ffi::status::OK {
        let err = payload
            .map(|message| {
                crate::error::from_swift(
                    status,
                    ffi::fm_string_dup(
                        CString::new(message)
                            .expect("stream errors must not contain NUL bytes")
                            .as_ptr(),
                    ),
                )
            })
            .unwrap_or_else(|| crate::error::from_swift(status, ptr::null_mut()));
        {
            let mut cb = state
                .on_event
                .lock()
                .expect("structured callback mutex poisoned");
            // Catch panics so they don't unwind across the FFI boundary (UB).
            let _ = catch_unwind(AssertUnwindSafe(|| {
                cb(StructuredStreamEvent::Error(err.clone()));
            }));
        }
        if let Some(tx) = state
            .done_tx
            .lock()
            .expect("structured done_tx mutex poisoned")
            .take()
        {
            let _ = tx.send(Err(err));
        }
        drop(Arc::from_raw(Arc::as_ptr(&state)));
        drop(state);
        return;
    }

    if let Some(payload) = payload {
        let snapshot: BridgeStructuredSnapshot = match serde_json::from_str(&payload) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                let err = FMError::DecodingFailure(error.to_string());
                {
                    let mut cb = state
                        .on_event
                        .lock()
                        .expect("structured callback mutex poisoned");
                    let _ = catch_unwind(AssertUnwindSafe(|| {
                        cb(StructuredStreamEvent::Error(err.clone()));
                    }));
                }
                if let Some(tx) = state
                    .done_tx
                    .lock()
                    .expect("structured done_tx mutex poisoned")
                    .take()
                {
                    let _ = tx.send(Err(err));
                }
                drop(Arc::from_raw(Arc::as_ptr(&state)));
                drop(state);
                return;
            }
        };
        let snapshot_event = StructuredStreamEvent::Snapshot(StructuredStreamSnapshot {
            content_json: snapshot.content.json,
            raw_content_json: snapshot.raw_content.json,
            is_complete: snapshot.is_complete,
        });
        let snapshot_panicked = {
            let mut cb = state
                .on_event
                .lock()
                .expect("structured callback mutex poisoned");
            // Catch panics so they don't unwind across the FFI boundary.
            catch_unwind(AssertUnwindSafe(|| cb(snapshot_event))).is_err()
        };
        if snapshot_panicked {
            if let Some(tx) = state
                .done_tx
                .lock()
                .expect("structured done_tx mutex poisoned")
                .take()
            {
                let _ = tx.send(Err(FMError::Unknown {
                    code: ffi::status::UNKNOWN,
                    message: "stream callback panicked".into(),
                }));
            }
            drop(Arc::from_raw(Arc::as_ptr(&state)));
            drop(state);
            return;
        }
    }

    if done {
        {
            let mut cb = state
                .on_event
                .lock()
                .expect("structured callback mutex poisoned");
            let _ = catch_unwind(AssertUnwindSafe(|| cb(StructuredStreamEvent::Done)));
        }
        if let Some(tx) = state
            .done_tx
            .lock()
            .expect("structured done_tx mutex poisoned")
            .take()
        {
            let _ = tx.send(Ok(()));
        }
        drop(Arc::from_raw(Arc::as_ptr(&state)));
    }
    drop(state);
}
