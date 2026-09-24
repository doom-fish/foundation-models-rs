//! [`LanguageModelSession`] — a stateful conversation with the on-device model.

use core::ffi::{c_char, c_void};
use core::ptr;
use std::ffi::CString;
use std::sync::mpsc;
use std::sync::{Arc, Mutex, PoisonError};

use doom_fish_utils::panic_safe::{catch_user_panic, catch_user_panic_result};
use serde::Deserialize;
use serde_json::json;

use crate::content::{BridgeGeneratedContent, GeneratedContent};
use crate::error::{bridge_text_result, from_swift_message, take_bridge_string, FMError};
use crate::ffi;
use crate::generation::{GenerationOptions, SamplingMode};
use crate::model::ConfiguredSystemLanguageModel;
use crate::prompt::{Instructions, Prompt, ToInstructions, ToPrompt};
use crate::schema::GenerationSchema;
use crate::task::SwiftTask;
use crate::tool::{
    release_tool_registry, tool_callback_trampoline, tool_specs_json, Tool, ToolRegistry,
};
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
/// let session = LanguageModelSession::new()?;
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
    /// Return the raw opaque pointer to the underlying Swift session object.
    ///
    /// Used internally by `async_api` to pass the session pointer to FFI
    /// callbacks without exposing `ptr` as a public field.
    #[cfg(feature = "async")]
    pub(crate) fn as_ptr(&self) -> *mut c_void {
        self.ptr
    }

    /// Create a session with the model's default behaviour.
    ///
    /// # Errors
    ///
    /// Returns an [`FMError`] if `FoundationModels` is not available on this OS.
    pub fn new() -> Result<Self, FMError> {
        Self::builder().build()
    }

    /// Create a session with custom system instructions ("system prompt").
    ///
    /// # Errors
    ///
    /// Returns an [`FMError`] if `FoundationModels` is not available.
    pub fn with_instructions(instructions: &str) -> Result<Self, FMError> {
        Self::builder().instructions(instructions)?.build()
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
            .map_err(|e| FMError::InvalidArgument(format!("prompt NUL byte: {e}").into()))?;
        let schema_c = CString::new(schema)
            .map_err(|e| FMError::InvalidArgument(format!("schema NUL byte: {e}").into()))?;
        let opts = options.validate()?.to_ffi();
        wait_for_bridge(|context, callback| unsafe {
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
                callback,
            )
        })
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
        run_text_stream_with(
            |context, callback| unsafe {
                ffi::fm_session_stream_request_json(self.ptr, payload.as_ptr(), context, callback)
            },
            on_chunk,
        )
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
            FMError::InvalidArgument(format!("prompt JSON contains a NUL byte: {error}").into())
        })?;
        let mut error: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::fm_session_prewarm_prompt_json(self.ptr, prompt_json.as_ptr(), &raw mut error)
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
        wait_for_bridge(|context, callback| unsafe {
            ffi::fm_session_respond_request_json(self.ptr, payload.as_ptr(), context, callback)
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
        wait_for_bridge(|context, callback| unsafe {
            ffi::fm_session_respond_request_json(self.ptr, payload.as_ptr(), context, callback)
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
            inner: Mutex::new(StructuredStreamInner {
                on_event: Box::new(on_event),
                done_tx: Some(done_tx),
            }),
        });
        let context = Arc::into_raw(state).cast_mut().cast::<c_void>();
        let task = SwiftTask::from_raw(unsafe {
            ffi::fm_session_stream_request_json(
                self.ptr,
                payload.as_ptr(),
                context,
                structured_stream_trampoline,
            )
        });
        let result = done_rx
            .recv()
            .unwrap_or_else(|_| Err(bridge_dropped("structured stream")));
        drop(task);
        result
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
            FMError::InvalidArgument(
                format!("feedback request contains a NUL byte: {error}").into(),
            )
        })?;
        let mut length = 0usize;
        let mut error: *mut c_char = ptr::null_mut();
        let ptr = unsafe {
            ffi::fm_session_log_feedback_attachment_json(
                self.ptr,
                request_json.as_ptr(),
                &raw mut length,
                &raw mut error,
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
        let tools_json = if self.tools.is_empty() {
            None
        } else {
            let registry = ToolRegistry::new(self.tools);
            Some((
                tool_specs_json(registry.tools.values())?,
                Arc::new(registry),
            ))
        };

        let instructions_c = instructions_json
            .as_deref()
            .map(CString::new)
            .transpose()
            .map_err(|error| {
                FMError::InvalidArgument(
                    format!("instructions JSON contains a NUL byte: {error}").into(),
                )
            })?;
        let transcript_c = transcript_json
            .as_deref()
            .map(CString::new)
            .transpose()
            .map_err(|error| {
                FMError::InvalidArgument(
                    format!("transcript JSON contains a NUL byte: {error}").into(),
                )
            })?;
        let (tools_c, tool_registry) = match tools_json {
            Some((json, registry)) => (
                Some(CString::new(json).map_err(|error| {
                    FMError::InvalidArgument(
                        format!("tool JSON contains a NUL byte: {error}").into(),
                    )
                })?),
                Some(registry),
            ),
            None => (None, None),
        };

        let has_tools = tool_registry.is_some();
        let tool_context = tool_registry.map_or(ptr::null_mut(), ToolRegistry::into_swift_context);
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
                has_tools.then_some(release_tool_registry as ffi::FmReleaseCallback),
                has_tools.then_some(tool_callback_trampoline as ffi::FmToolCallback),
                &raw mut error,
            )
        };
        if ptr.is_null() {
            return Err(crate::error::from_swift(
                ffi::status::MODEL_UNAVAILABLE,
                error,
            ));
        }
        Ok(LanguageModelSession { ptr })
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
            FMError::InvalidArgument(
                format!("feedback request is not JSON-serializable: {error}").into(),
            )
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
    content: String,
}

pub(crate) fn respond_request_json(
    prompt: &Prompt,
    options: GenerationOptions,
    schema: Option<&GenerationSchema>,
    include_schema_in_prompt: bool,
) -> Result<CString, FMError> {
    let options = options.validate()?;
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
    .map_err(|error| {
        FMError::InvalidArgument(format!("request is not JSON-serializable: {error}").into())
    })?;
    CString::new(payload).map_err(|error| {
        FMError::InvalidArgument(format!("request JSON contains a NUL byte: {error}").into())
    })
}

pub(crate) trait BridgePayload: Sized + Send + 'static {
    fn decode(payload: String) -> Result<Self, FMError>;
}

impl BridgePayload for String {
    fn decode(payload: String) -> Result<Self, FMError> {
        Ok(payload)
    }
}

impl BridgePayload for SessionResponse<String> {
    fn decode(payload: String) -> Result<Self, FMError> {
        let response: BridgeTextResponse = serde_json::from_str(&payload)
            .map_err(|error| FMError::DecodingFailure(error.to_string().into()))?;
        Ok(Self {
            content: response.content,
            raw_content: GeneratedContent::from_bridge_payload(response.raw_content, true)?,
            transcript: Transcript::from_json_str(&response.transcript_json)?,
        })
    }
}

impl BridgePayload for SessionResponse<GeneratedContent> {
    fn decode(payload: String) -> Result<Self, FMError> {
        let response: BridgeStructuredResponse = serde_json::from_str(&payload)
            .map_err(|error| FMError::DecodingFailure(error.to_string().into()))?;
        Ok(Self {
            content: GeneratedContent::from_bridge_payload(response.content, true)?,
            raw_content: GeneratedContent::from_bridge_payload(response.raw_content, true)?,
            transcript: Transcript::from_json_str(&response.transcript_json)?,
        })
    }
}

fn bridge_dropped(what: &str) -> FMError {
    FMError::Unknown {
        code: ffi::status::UNKNOWN,
        message: format!("Swift bridge dropped the {what} callback").into(),
    }
}

fn callback_panicked() -> FMError {
    FMError::Unknown {
        code: ffi::status::UNKNOWN,
        message: "stream callback panicked".into(),
    }
}

pub(crate) fn wait_for_bridge<T, F>(invoke: F) -> Result<T, FMError>
where
    T: BridgePayload,
    F: FnOnce(*mut c_void, ffi::FmRespondCallback) -> *mut c_void,
{
    let (tx, rx) = mpsc::channel::<Result<T, FMError>>();
    let context = Box::into_raw(Box::new(tx)).cast::<c_void>();
    let _task = SwiftTask::from_raw(invoke(context, respond_trampoline::<T>));
    rx.recv()
        .unwrap_or_else(|_| Err(bridge_dropped("response")))
}

pub(crate) fn run_text_stream_with<F, C>(invoke: F, on_chunk: C) -> Result<(), FMError>
where
    F: FnOnce(*mut c_void, ffi::FmStreamCallback) -> *mut c_void,
    C: FnMut(StreamEvent<'_>) + Send + 'static,
{
    let (done_tx, done_rx) = mpsc::channel::<Result<(), FMError>>();
    let state = Arc::new(TextStreamState {
        inner: Mutex::new(TextStreamInner {
            on_chunk: Box::new(on_chunk),
            received: String::new(),
            done_tx: Some(done_tx),
        }),
    });
    let context = Arc::into_raw(state).cast_mut().cast::<c_void>();
    let _task = SwiftTask::from_raw(invoke(context, json_text_stream_trampoline));
    done_rx
        .recv()
        .unwrap_or_else(|_| Err(bridge_dropped("stream")))
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
    /// Incremental text delta. Concatenate these, starting from the text of the
    /// latest `Replace`, to reconstruct the full reply.
    Chunk(&'a str),
    Replace(&'a str),
    /// Stream finished successfully.
    Done,
    /// Stream failed; the inner error describes why.
    Error(FMError),
}

// ---------- internal callback plumbing ----------

// SAFETY: `context` is a `Box<mpsc::Sender<...>>` raw pointer created by
// `wait_for_bridge`. Swift calls this callback exactly once, so there is
// no double-free risk. `response` and `error` are heap-allocated C strings
// that this callback takes ownership of and frees.
unsafe extern "C" fn respond_trampoline<T: BridgePayload>(
    context: *mut c_void,
    response: *mut c_char,
    error: *mut c_char,
    status: i32,
) {
    let result = catch_user_panic_result("respond callback", || {
        unsafe { bridge_text_result(response, error, status) }.and_then(T::decode)
    })
    .unwrap_or_else(|| Err(bridge_dropped("response")));
    if context.is_null() {
        return;
    }
    let tx = unsafe { Box::from_raw(context.cast::<mpsc::Sender<Result<T, FMError>>>()) };
    let _ = tx.send(result);
}

enum TextUpdate<'a> {
    Unchanged,
    Append(&'a str),
    Replace(&'a str),
}

fn text_update<'a>(received: &str, snapshot: &'a str) -> TextUpdate<'a> {
    match snapshot.strip_prefix(received) {
        Some("") => TextUpdate::Unchanged,
        Some(delta) => TextUpdate::Append(delta),
        None => TextUpdate::Replace(snapshot),
    }
}

type StreamCallback = Box<dyn FnMut(StreamEvent<'_>) + Send>;

struct TextStreamState {
    inner: Mutex<TextStreamInner>,
}

struct TextStreamInner {
    on_chunk: StreamCallback,
    received: String,
    done_tx: Option<mpsc::Sender<Result<(), FMError>>>,
}

impl TextStreamInner {
    fn emit(&mut self, event: StreamEvent<'_>) -> bool {
        catch_user_panic_result("stream callback", || (self.on_chunk)(event)).is_some()
    }

    fn finish(&mut self, result: Result<(), FMError>) {
        if let Some(done_tx) = self.done_tx.take() {
            let _ = done_tx.send(result);
        }
    }

    fn fail(&mut self, error: FMError) {
        self.emit(StreamEvent::Error(error.clone()));
        self.finish(Err(error));
    }

    fn deliver(&mut self, payload: Option<String>, done: bool, status: i32) {
        if self.done_tx.is_none() {
            return;
        }
        if status != ffi::status::OK {
            self.fail(from_swift_message(status, payload.unwrap_or_default()));
            return;
        }
        if let Some(payload) = payload {
            let snapshot = match serde_json::from_str::<BridgeTextStreamSnapshot>(&payload) {
                Ok(snapshot) => snapshot,
                Err(error) => {
                    self.fail(FMError::DecodingFailure(error.to_string().into()));
                    return;
                }
            };
            let delivered = match text_update(&self.received, &snapshot.content) {
                TextUpdate::Unchanged => true,
                TextUpdate::Append(delta) => self.emit(StreamEvent::Chunk(delta)),
                TextUpdate::Replace(text) => self.emit(StreamEvent::Replace(text)),
            };
            self.received = snapshot.content;
            if !delivered {
                self.finish(Err(callback_panicked()));
                return;
            }
        }
        if done {
            self.emit(StreamEvent::Done);
            self.finish(Ok(()));
        }
    }
}

// SAFETY: `context` is an `Arc<TextStreamState>` raw pointer passed via
// `Arc::into_raw`. Swift sends exactly one terminal callback (`done == true`
// or `status != OK`), and only that call releases the reference, so every
// earlier call sees a live state. Once the waiter has been answered (normal
// end, error, or callback panic) later calls are ignored until the terminal
// one arrives. `chunk` is a heap-allocated C string this callback frees.
unsafe extern "C" fn json_text_stream_trampoline(
    context: *mut c_void,
    chunk: *mut c_char,
    done: bool,
    status: i32,
) {
    let payload = unsafe { take_bridge_string(chunk) };
    if context.is_null() {
        return;
    }
    let state = context.cast_const().cast::<TextStreamState>();
    catch_user_panic("text stream callback", || {
        unsafe { &*state }
            .inner
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .deliver(payload, done, status);
    });
    if done || status != ffi::status::OK {
        catch_user_panic("text stream release", || unsafe {
            drop(Arc::from_raw(state))
        });
    }
}

type StructuredStreamCallback = Box<dyn FnMut(StructuredStreamEvent) + Send>;

struct StructuredStreamState {
    inner: Mutex<StructuredStreamInner>,
}

struct StructuredStreamInner {
    on_event: StructuredStreamCallback,
    done_tx: Option<mpsc::Sender<Result<(), FMError>>>,
}

impl StructuredStreamInner {
    fn emit(&mut self, event: StructuredStreamEvent) -> bool {
        catch_user_panic_result("structured stream callback", || (self.on_event)(event)).is_some()
    }

    fn finish(&mut self, result: Result<(), FMError>) {
        if let Some(done_tx) = self.done_tx.take() {
            let _ = done_tx.send(result);
        }
    }

    fn fail(&mut self, error: FMError) {
        self.emit(StructuredStreamEvent::Error(error.clone()));
        self.finish(Err(error));
    }

    fn deliver(&mut self, payload: Option<String>, done: bool, status: i32) {
        if self.done_tx.is_none() {
            return;
        }
        if status != ffi::status::OK {
            self.fail(from_swift_message(status, payload.unwrap_or_default()));
            return;
        }
        if let Some(payload) = payload {
            let snapshot = match serde_json::from_str::<BridgeStructuredSnapshot>(&payload) {
                Ok(snapshot) => snapshot,
                Err(error) => {
                    self.fail(FMError::DecodingFailure(error.to_string().into()));
                    return;
                }
            };
            let event = StructuredStreamEvent::Snapshot(StructuredStreamSnapshot {
                content_json: snapshot.content.json,
                raw_content_json: snapshot.raw_content.json,
                is_complete: snapshot.is_complete,
            });
            if !self.emit(event) {
                self.finish(Err(callback_panicked()));
                return;
            }
        }
        if done {
            self.emit(StructuredStreamEvent::Done);
            self.finish(Ok(()));
        }
    }
}

// SAFETY: Same invariants as `json_text_stream_trampoline` above, but for
// `StructuredStreamState`.
unsafe extern "C" fn structured_stream_trampoline(
    context: *mut c_void,
    chunk: *mut c_char,
    done: bool,
    status: i32,
) {
    let payload = unsafe { take_bridge_string(chunk) };
    if context.is_null() {
        return;
    }
    let state = context.cast_const().cast::<StructuredStreamState>();
    catch_user_panic("structured stream callback", || {
        unsafe { &*state }
            .inner
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .deliver(payload, done, status);
    });
    if done || status != ffi::status::OK {
        catch_user_panic("structured stream release", || unsafe {
            drop(Arc::from_raw(state));
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::{Duration, Instant};

    use crate::tool::ToolOutput;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Seen {
        Chunk(String),
        Replace(String),
        Done,
        Error(i32),
    }

    fn bridge_string(value: &str) -> *mut c_char {
        let value = CString::new(value).unwrap();
        unsafe { ffi::fm_string_dup(value.as_ptr()) }
    }

    fn snapshot(content: &str) -> *mut c_char {
        bridge_string(&json!({ "kind": "text", "content": content }).to_string())
    }

    struct TestStream {
        context: *mut c_void,
        weak: std::sync::Weak<TextStreamState>,
        seen: Arc<Mutex<Vec<Seen>>>,
        done_rx: mpsc::Receiver<Result<(), FMError>>,
    }

    fn text_stream(panic_on_chunk: bool) -> TestStream {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let recorder = Arc::clone(&seen);
        let (done_tx, done_rx) = mpsc::channel();
        let state = Arc::new(TextStreamState {
            inner: Mutex::new(TextStreamInner {
                on_chunk: Box::new(move |event| {
                    let event = match event {
                        StreamEvent::Chunk(delta) => Seen::Chunk(delta.to_owned()),
                        StreamEvent::Replace(text) => Seen::Replace(text.to_owned()),
                        StreamEvent::Done => Seen::Done,
                        StreamEvent::Error(error) => Seen::Error(error.code()),
                    };
                    let is_chunk = matches!(event, Seen::Chunk(_));
                    recorder.lock().unwrap().push(event);
                    assert!(
                        !(panic_on_chunk && is_chunk),
                        "callback panicked on purpose"
                    );
                }),
                received: String::new(),
                done_tx: Some(done_tx),
            }),
        });
        TestStream {
            weak: Arc::downgrade(&state),
            context: Arc::into_raw(state).cast_mut().cast(),
            seen,
            done_rx,
        }
    }

    fn reassemble(events: &[Seen]) -> String {
        let mut text = String::new();
        for event in events {
            match event {
                Seen::Chunk(delta) => text.push_str(delta),
                Seen::Replace(replacement) => replacement.clone_into(&mut text),
                Seen::Done | Seen::Error(_) => {}
            }
        }
        text
    }

    fn feed(snapshots: &[&str]) -> Vec<Seen> {
        let TestStream {
            context,
            weak,
            seen,
            done_rx,
        } = text_stream(false);
        for content in snapshots {
            unsafe {
                json_text_stream_trampoline(context, snapshot(content), false, ffi::status::OK)
            };
        }
        unsafe {
            json_text_stream_trampoline(context, ptr::null_mut(), true, ffi::status::OK);
        }
        assert_eq!(done_rx.recv().unwrap(), Ok(()));
        assert!(weak.upgrade().is_none());
        let events = seen.lock().unwrap().clone();
        assert_eq!(events.last(), Some(&Seen::Done));
        assert_eq!(reassemble(&events), *snapshots.last().unwrap());
        events
    }

    #[test]
    fn deltas_split_growing_grapheme_clusters_on_scalar_boundaries() {
        let family = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}";
        let events = feed(&[
            "Hi \u{1F468}",
            "Hi \u{1F468}\u{200D}",
            "Hi \u{1F468}\u{200D}\u{1F469}",
            &format!("Hi {family}"),
            &format!("Hi {family} and \u{1F44D}"),
            &format!("Hi {family} and \u{1F44D}\u{1F3FD}"),
            &format!("Hi {family} and \u{1F44D}\u{1F3FD} cafe"),
            &format!("Hi {family} and \u{1F44D}\u{1F3FD} cafe\u{301}"),
            &format!("Hi {family} and \u{1F44D}\u{1F3FD} cafe\u{301} \u{2764}"),
            &format!("Hi {family} and \u{1F44D}\u{1F3FD} cafe\u{301} \u{2764}\u{FE0F}"),
        ]);
        assert!(events
            .iter()
            .all(|event| !matches!(event, Seen::Replace(_))));
        assert!(events.contains(&Seen::Chunk("\u{200D}".into())));
        assert!(events.contains(&Seen::Chunk("\u{1F3FD}".into())));
        assert!(events.contains(&Seen::Chunk("\u{301}".into())));
        assert!(events.contains(&Seen::Chunk("\u{FE0F}".into())));
    }

    #[test]
    fn rewritten_snapshots_are_reported_as_replacements() {
        let events = feed(&["The cat", "The cat", "The dog", "The dog sat"]);
        assert_eq!(
            events,
            vec![
                Seen::Chunk("The cat".into()),
                Seen::Replace("The dog".into()),
                Seen::Chunk(" sat".into()),
                Seen::Done,
            ]
        );
    }

    #[test]
    fn a_panicking_callback_finishes_the_stream_but_keeps_the_state_until_swift_is_done() {
        let TestStream {
            context,
            weak,
            seen,
            done_rx,
        } = text_stream(true);
        unsafe { json_text_stream_trampoline(context, snapshot("first"), false, ffi::status::OK) };
        assert_eq!(done_rx.recv().unwrap(), Err(callback_panicked()));

        unsafe {
            json_text_stream_trampoline(context, snapshot("first second"), false, ffi::status::OK)
        };
        assert!(weak.upgrade().is_some());

        let cancelled = bridge_string(&json!({ "message": "generation cancelled" }).to_string());
        unsafe { json_text_stream_trampoline(context, cancelled, true, ffi::status::CANCELLED) };
        assert!(weak.upgrade().is_none());
        assert_eq!(*seen.lock().unwrap(), vec![Seen::Chunk("first".into())]);
    }

    #[test]
    fn stream_errors_are_typed_and_delivered_once() {
        let TestStream {
            context,
            weak,
            seen,
            done_rx,
        } = text_stream(false);
        let payload = bridge_string(&json!({ "message": "blocked" }).to_string());
        unsafe {
            json_text_stream_trampoline(context, payload, true, ffi::status::GUARDRAIL_VIOLATION)
        };
        assert!(
            matches!(done_rx.recv().unwrap(), Err(FMError::GuardrailViolation(message)) if message == "blocked")
        );
        assert!(weak.upgrade().is_none());
        assert_eq!(
            *seen.lock().unwrap(),
            vec![Seen::Error(ffi::status::GUARDRAIL_VIOLATION)]
        );
    }

    #[test]
    fn undecodable_snapshots_fail_the_stream_without_freeing_it() {
        let TestStream {
            context,
            weak,
            done_rx,
            ..
        } = text_stream(false);
        unsafe {
            json_text_stream_trampoline(context, bridge_string("not json"), false, ffi::status::OK)
        };
        assert!(matches!(
            done_rx.recv().unwrap(),
            Err(FMError::DecodingFailure(_))
        ));
        assert!(weak.upgrade().is_some());
        unsafe { json_text_stream_trampoline(context, ptr::null_mut(), true, ffi::status::OK) };
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn structured_stream_state_is_released_only_by_the_terminal_callback() {
        let (done_tx, done_rx) = mpsc::channel();
        let snapshots = Arc::new(Mutex::new(0_usize));
        let counter = Arc::clone(&snapshots);
        let state = Arc::new(StructuredStreamState {
            inner: Mutex::new(StructuredStreamInner {
                on_event: Box::new(move |event| {
                    if let StructuredStreamEvent::Snapshot(snapshot) = event {
                        *counter.lock().unwrap() += 1;
                        assert!(snapshot.is_complete, "callback panicked on purpose");
                    }
                }),
                done_tx: Some(done_tx),
            }),
        });
        let weak = Arc::downgrade(&state);
        let context = Arc::into_raw(state).cast_mut().cast::<c_void>();
        let partial = json!({
            "kind": "generated_content",
            "content": { "json": "{}" },
            "rawContent": { "json": "{}" },
            "isComplete": false,
        });
        unsafe {
            structured_stream_trampoline(
                context,
                bridge_string(&partial.to_string()),
                false,
                ffi::status::OK,
            );
        }
        assert_eq!(done_rx.recv().unwrap(), Err(callback_panicked()));
        unsafe {
            structured_stream_trampoline(
                context,
                bridge_string(&partial.to_string()),
                false,
                ffi::status::OK,
            );
        }
        assert!(weak.upgrade().is_some());
        unsafe { structured_stream_trampoline(context, ptr::null_mut(), true, ffi::status::OK) };
        assert!(weak.upgrade().is_none());
        assert_eq!(*snapshots.lock().unwrap(), 1);
    }

    #[test]
    fn respond_requests_reject_invalid_sampling_before_calling_swift() {
        let error = respond_request_json(
            &Prompt::from("hello"),
            GenerationOptions::new().with_sampling(SamplingMode::TopK(0)),
            None,
            true,
        )
        .expect_err("top-k 0 must be rejected");
        assert!(matches!(error, FMError::InvalidArgument(_)));
    }

    struct DropFlag(Arc<AtomicBool>);

    impl Drop for DropFlag {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    #[test]
    fn the_swift_session_owns_the_tool_registry() {
        let dropped = Arc::new(AtomicBool::new(false));
        let flag = DropFlag(Arc::clone(&dropped));
        let tool = Tool::new(
            "probe",
            "Report that the registry is alive.",
            GenerationSchema::generated_content(),
            move |_| {
                let _ = &flag;
                Ok(ToolOutput::text("alive"))
            },
        );
        let session = match LanguageModelSession::builder().tool(tool).build() {
            Ok(session) => session,
            Err(error) => {
                eprintln!("SKIP: cannot create a session here: {error}");
                return;
            }
        };
        let swift_session = session.ptr;
        unsafe { ffi::fm_object_retain(swift_session) };
        drop(session);
        assert!(
            !dropped.load(Ordering::SeqCst),
            "the registry must outlive the Rust session while Swift holds the session"
        );

        unsafe { ffi::fm_object_release(swift_session) };
        let deadline = Instant::now() + Duration::from_secs(10);
        while !dropped.load(Ordering::SeqCst) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(
            dropped.load(Ordering::SeqCst),
            "releasing the last Swift reference must free the registry"
        );
    }

    #[test]
    fn instructions_with_nul_bytes_build_a_session() {
        let result = LanguageModelSession::builder()
            .instructions("Answer\0briefly")
            .and_then(SessionBuilder::build);
        if let Err(FMError::ModelUnavailable { .. }) = result {
            eprintln!("SKIP: FoundationModels unavailable");
            return;
        }
        assert!(result.is_ok(), "{result:?}");
        assert!(LanguageModelSession::with_instructions("Answer\0briefly").is_ok());
    }
}
