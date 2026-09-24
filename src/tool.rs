//! Tool calling support.

use core::ffi::{c_char, c_void};
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::sync::Arc;

use doom_fish_utils::panic_safe::{catch_user_panic, catch_user_panic_result};
use serde::de::DeserializeOwned;
use serde_json::json;

use crate::content::{FromGeneratedContent, GeneratedContent};
use crate::error::FMError;
use crate::ffi;
use crate::prompt::{Prompt, Segment, ToPrompt, ToolDefinition};
use crate::schema::{Generable, GenerationSchema};

fn swift_dup_string(value: &str) -> *mut c_char {
    let c_string = CString::new(value)
        .or_else(|_| CString::new(value.replace('\0', "\u{FFFD}")))
        .unwrap_or_default();
    unsafe { ffi::fm_string_dup(c_string.as_ptr()) }
}

/// One tool exposed to the system language model.
pub struct Tool {
    spec: ToolSpec,
    handler: Arc<dyn Fn(GeneratedContent) -> Result<ToolOutput, FMError> + Send + Sync>,
}

impl Tool {
    /// Create a tool from a dynamic `GeneratedContent` handler.
    #[must_use]
    pub fn new<F>(
        name: impl Into<String>,
        description: impl Into<String>,
        parameters: GenerationSchema,
        handler: F,
    ) -> Self
    where
        F: Fn(GeneratedContent) -> Result<ToolOutput, FMError> + Send + Sync + 'static,
    {
        Self {
            spec: ToolSpec {
                name: name.into(),
                description: description.into(),
                parameters,
                includes_schema_in_instructions: true,
            },
            handler: Arc::new(handler),
        }
    }

    /// Create a tool whose handler receives decoded JSON arguments.
    #[must_use]
    pub fn json<Args, Output, F>(
        name: impl Into<String>,
        description: impl Into<String>,
        parameters: GenerationSchema,
        handler: F,
    ) -> Self
    where
        Args: DeserializeOwned + Send + 'static,
        Output: ToPrompt,
        F: Fn(Args) -> Result<Output, FMError> + Send + Sync + 'static,
    {
        Self::new(name, description, parameters, move |arguments| {
            let decoded = arguments.value::<Args>()?;
            let output = handler(decoded)?;
            Ok(ToolOutput::from_prompt(output.to_prompt()?))
        })
    }

    /// Create a tool whose argument schema is inferred from a [`Generable`] type.
    ///
    /// # Errors
    ///
    /// Returns an [`FMError`] if `Args` cannot produce a generation schema.
    pub fn generable<Args, Output, F>(
        name: impl Into<String>,
        description: impl Into<String>,
        handler: F,
    ) -> Result<Self, FMError>
    where
        Args: FromGeneratedContent + Generable + Send + 'static,
        Output: ToPrompt,
        F: Fn(Args) -> Result<Output, FMError> + Send + Sync + 'static,
    {
        Ok(Self::new(
            name,
            description,
            Args::generation_schema()?,
            move |arguments| {
                let decoded = Args::from_generated_content(&arguments)?;
                let output = handler(decoded)?;
                Ok(ToolOutput::from_prompt(output.to_prompt()?))
            },
        ))
    }

    /// Tool metadata as exposed to FoundationModels.
    #[must_use]
    pub const fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    /// Convert this tool into a transcript tool definition.
    #[must_use]
    pub fn definition(&self) -> ToolDefinition {
        self.spec.definition()
    }

    /// Control whether the schema is included in the model's tool instructions.
    #[must_use]
    pub fn with_schema_in_instructions(mut self, includes: bool) -> Self {
        self.spec.includes_schema_in_instructions = includes;
        self
    }
}

impl core::fmt::Debug for Tool {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Tool").field("spec", &self.spec).finish()
    }
}

/// Public metadata for one tool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub parameters: GenerationSchema,
    pub includes_schema_in_instructions: bool,
}

impl ToolSpec {
    /// Convert this tool specification into a transcript tool definition.
    #[must_use]
    pub fn definition(&self) -> ToolDefinition {
        ToolDefinition::new(
            self.name.clone(),
            self.description.clone(),
            self.parameters.clone(),
        )
    }
}

/// A tool output converted into a prompt representation.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolOutput {
    prompt: Prompt,
}

impl ToolOutput {
    /// Return a tool output as plain text.
    #[must_use]
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            prompt: Prompt::from(text.into()),
        }
    }

    /// Return a tool output as structured content.
    #[must_use]
    pub fn structured(content: GeneratedContent) -> Self {
        Self {
            prompt: Prompt::from(content),
        }
    }

    /// Return a prebuilt prompt output.
    #[must_use]
    pub const fn from_prompt(prompt: Prompt) -> Self {
        Self { prompt }
    }

    #[must_use]
    pub fn prompt(&self) -> &Prompt {
        &self.prompt
    }

    pub(crate) fn to_bridge_json(&self) -> Result<String, FMError> {
        serde_json::to_string(&json!({ "prompt": self.prompt.to_bridge_value() })).map_err(
            |error| {
                FMError::InvalidArgument(
                    format!("tool output is not JSON-serializable: {error}").into(),
                )
            },
        )
    }

    fn hand_generation_ids_to_swift(&self) {
        for segment in self.prompt.segments() {
            if let Segment::Structure(segment) = segment {
                if let Some(generation_id) = segment.content.generation_id() {
                    unsafe { ffi::fm_generation_id_retain(generation_id.token()) };
                }
            }
        }
    }
}

impl From<String> for ToolOutput {
    fn from(value: String) -> Self {
        Self::text(value)
    }
}

impl From<&str> for ToolOutput {
    fn from(value: &str) -> Self {
        Self::text(value)
    }
}

impl From<GeneratedContent> for ToolOutput {
    fn from(value: GeneratedContent) -> Self {
        Self::structured(value)
    }
}

impl From<Prompt> for ToolOutput {
    fn from(value: Prompt) -> Self {
        Self::from_prompt(value)
    }
}

pub(crate) fn tool_specs_json<'a>(
    tools: impl IntoIterator<Item = &'a Tool>,
) -> Result<String, FMError> {
    let specs = tools
        .into_iter()
        .map(|tool| {
            json!({
                "name": tool.spec.name,
                "description": tool.spec.description,
                "parametersJSON": tool.spec.parameters.bridge_request_json(),
                "includesSchemaInInstructions": tool.spec.includes_schema_in_instructions,
            })
        })
        .collect::<Vec<_>>();
    serde_json::to_string(&specs).map_err(|error| {
        FMError::InvalidArgument(format!("tool specs are not JSON-serializable: {error}").into())
    })
}

pub(crate) struct ToolRegistry {
    pub(crate) tools: HashMap<String, Tool>,
}

impl ToolRegistry {
    pub(crate) fn new(tools: Vec<Tool>) -> Self {
        Self {
            tools: tools
                .into_iter()
                .map(|tool| (tool.spec.name.clone(), tool))
                .collect(),
        }
    }

    pub(crate) fn into_swift_context(self: Arc<Self>) -> *mut c_void {
        Arc::into_raw(self).cast_mut().cast()
    }

    fn invoke(&self, tool_name: &str, arguments: GeneratedContent) -> Result<ToolOutput, FMError> {
        let tool = self.tools.get(tool_name).ok_or_else(|| {
            FMError::ToolCallFailed(format!("tool `{tool_name}` is not registered").into())
        })?;
        (tool.handler)(arguments)
    }
}

pub(crate) unsafe extern "C" fn release_tool_registry(context: *mut c_void) {
    if context.is_null() {
        return;
    }
    catch_user_panic("tool registry release", || unsafe {
        drop(Arc::from_raw(context.cast_const().cast::<ToolRegistry>()));
    });
}

// SAFETY: `context` is the Swift-owned `Arc<ToolRegistry>` reference, which
// the calling `RustTool` keeps alive for the duration of this call.
// `tool_name` and `arguments_json` are NUL-terminated UTF-8 C strings owned by
// the Swift bridge and valid for the duration of this call.
pub(crate) unsafe extern "C" fn tool_callback_trampoline(
    context: *mut c_void,
    tool_name: *const c_char,
    arguments_json: *const c_char,
    output_json_out: *mut *mut c_char,
    error_out: *mut *mut c_char,
) -> i32 {
    let outcome = catch_user_panic_result("tool callback", || {
        if context.is_null() || tool_name.is_null() || arguments_json.is_null() {
            return Err(FMError::ToolCallFailed(
                "tool callback received a null argument".into(),
            ));
        }
        let registry = unsafe { &*context.cast_const().cast::<ToolRegistry>() };
        let tool_name = unsafe { CStr::from_ptr(tool_name) }.to_string_lossy();
        let arguments_json = unsafe { CStr::from_ptr(arguments_json) }.to_string_lossy();
        let arguments = GeneratedContent::from_json_str(&arguments_json)?;
        let output = registry.invoke(&tool_name, arguments)?;
        let output_json = output.to_bridge_json()?;
        Ok((output, output_json))
    });

    let (status, output, message) = match outcome {
        Some(Ok(output)) => (ffi::status::OK, Some(output), None),
        Some(Err(error)) => (error.code(), None, Some(error.message().to_owned())),
        None => (
            ffi::status::TOOL_CALL_FAILED,
            None,
            Some("tool callback panicked".to_owned()),
        ),
    };
    if let (Some((output, output_json)), false) = (output, output_json_out.is_null()) {
        unsafe { *output_json_out = swift_dup_string(&output_json) };
        output.hand_generation_ids_to_swift();
    }
    if let (Some(message), false) = (message, error_out.is_null()) {
        unsafe { *error_out = swift_dup_string(&message) };
    }
    status
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::ptr;
    use std::sync::atomic::{AtomicBool, Ordering};

    struct DropFlag(Arc<AtomicBool>);

    impl Drop for DropFlag {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    fn registry_with(
        handler: impl Fn(GeneratedContent) -> Result<ToolOutput, FMError> + Send + Sync + 'static,
    ) -> Arc<ToolRegistry> {
        Arc::new(ToolRegistry::new(vec![Tool::new(
            "echo",
            "Echo the input.",
            GenerationSchema::generated_content(),
            handler,
        )]))
    }

    fn take(ptr: *mut c_char) -> Option<String> {
        if ptr.is_null() {
            return None;
        }
        let value = unsafe { CStr::from_ptr(ptr) }
            .to_string_lossy()
            .into_owned();
        unsafe { ffi::fm_string_free(ptr) };
        Some(value)
    }

    fn call(
        registry: &Arc<ToolRegistry>,
        name: &str,
        arguments: &str,
    ) -> (i32, Option<String>, Option<String>) {
        let name = CString::new(name).unwrap();
        let arguments = CString::new(arguments).unwrap();
        let mut output: *mut c_char = ptr::null_mut();
        let mut error: *mut c_char = ptr::null_mut();
        let status = unsafe {
            tool_callback_trampoline(
                Arc::as_ptr(registry).cast_mut().cast(),
                name.as_ptr(),
                arguments.as_ptr(),
                &raw mut output,
                &raw mut error,
            )
        };
        (status, take(output), take(error))
    }

    #[test]
    fn swift_dup_string_replaces_interior_nul() {
        assert_eq!(
            take(swift_dup_string("a\0b")).as_deref(),
            Some("a\u{FFFD}b")
        );
        assert_eq!(take(swift_dup_string("plain")).as_deref(), Some("plain"));
    }

    #[test]
    fn tool_errors_with_nul_bytes_reach_swift_without_panicking() {
        let registry = registry_with(|_| Err(FMError::ToolCallFailed("bad\0argument".into())));
        let (status, output, error) = call(&registry, "echo", "{}");
        assert_eq!(status, ffi::status::TOOL_CALL_FAILED);
        assert_eq!(output, None);
        assert_eq!(error.as_deref(), Some("bad\u{FFFD}argument"));
    }

    #[test]
    fn panicking_tools_report_an_error() {
        let registry = registry_with(|_| panic!("tool exploded"));
        let (status, output, error) = call(&registry, "echo", "{}");
        assert_eq!(status, ffi::status::TOOL_CALL_FAILED);
        assert_eq!(output, None);
        assert_eq!(error.as_deref(), Some("tool callback panicked"));
    }

    #[test]
    fn unknown_tools_and_bad_arguments_fail_cleanly() {
        let registry = registry_with(|_| Ok(ToolOutput::text("ok")));
        let (status, _, error) = call(&registry, "missing", "{}");
        assert_eq!(status, ffi::status::TOOL_CALL_FAILED);
        assert!(error.unwrap().contains("not registered"));

        let (status, output, _) = call(&registry, "echo", "not json");
        assert_ne!(status, ffi::status::OK);
        assert_eq!(output, None);

        let (status, output, error) = call(&registry, "echo", "{}");
        assert_eq!(status, ffi::status::OK);
        assert!(output.unwrap().contains("ok"));
        assert_eq!(error, None);
    }

    #[test]
    fn null_arguments_are_rejected() {
        let registry = registry_with(|_| Ok(ToolOutput::text("ok")));
        let mut output: *mut c_char = ptr::null_mut();
        let mut error: *mut c_char = ptr::null_mut();
        let status = unsafe {
            tool_callback_trampoline(
                Arc::as_ptr(&registry).cast_mut().cast(),
                ptr::null(),
                ptr::null(),
                &raw mut output,
                &raw mut error,
            )
        };
        assert_eq!(status, ffi::status::TOOL_CALL_FAILED);
        assert!(output.is_null());
        assert!(take(error).is_some());
    }

    #[test]
    fn swift_owned_reference_keeps_the_registry_alive() {
        let dropped = Arc::new(AtomicBool::new(false));
        let flag = DropFlag(Arc::clone(&dropped));
        let registry = registry_with(move |_| {
            let _ = &flag;
            Ok(ToolOutput::text("still alive"))
        });
        let context = Arc::clone(&registry).into_swift_context();
        drop(registry);
        assert!(!dropped.load(Ordering::SeqCst));

        let registry = unsafe { &*context.cast_const().cast::<ToolRegistry>() };
        let output = registry
            .invoke("echo", GeneratedContent::from_json_str("{}").unwrap())
            .unwrap();
        assert_eq!(output.prompt(), &Prompt::text("still alive"));

        unsafe { release_tool_registry(context) };
        assert!(dropped.load(Ordering::SeqCst));
        unsafe { release_tool_registry(ptr::null_mut()) };
    }
}
