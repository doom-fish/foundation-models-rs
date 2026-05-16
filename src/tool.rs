//! Tool calling support.

use core::ffi::{c_char, c_void};
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Arc;

use serde::de::DeserializeOwned;
use serde_json::json;

use crate::content::GeneratedContent;
use crate::error::FMError;
use crate::ffi;
use crate::prompt::{Prompt, ToPrompt};
use crate::schema::GenerationSchema;

fn swift_dup_string(value: &str) -> *mut c_char {
    let c_string = CString::new(value).expect("bridge strings must not contain interior NUL bytes");
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
                FMError::InvalidArgument(format!("tool output is not JSON-serializable: {error}"))
            },
        )
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

pub(crate) struct ToolRegistry {
    tools: HashMap<String, Tool>,
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

    pub(crate) fn specs_json(&self) -> Result<String, FMError> {
        let specs = self
            .tools
            .values()
            .map(|tool| {
                json!({
                    "name": tool.spec.name,
                    "description": tool.spec.description,
                    "parametersJSON": tool.spec.parameters.json_schema(),
                    "includesSchemaInInstructions": tool.spec.includes_schema_in_instructions,
                })
            })
            .collect::<Vec<_>>();
        serde_json::to_string(&specs).map_err(|error| {
            FMError::InvalidArgument(format!("tool specs are not JSON-serializable: {error}"))
        })
    }

    fn invoke(&self, tool_name: &str, arguments: GeneratedContent) -> Result<ToolOutput, FMError> {
        let tool = self.tools.get(tool_name).ok_or_else(|| {
            FMError::ToolCallFailed(format!("tool `{tool_name}` is not registered"))
        })?;
        (tool.handler)(arguments)
    }
}

pub(crate) unsafe extern "C" fn tool_callback_trampoline(
    context: *mut c_void,
    tool_name: *const c_char,
    arguments_json: *const c_char,
    output_json_out: *mut *mut c_char,
    error_out: *mut *mut c_char,
) -> i32 {
    let registry = &*(context.cast::<ToolRegistry>());
    let result = catch_unwind(AssertUnwindSafe(|| {
        let tool_name = CStr::from_ptr(tool_name).to_string_lossy().into_owned();
        let arguments_json = CStr::from_ptr(arguments_json)
            .to_string_lossy()
            .into_owned();
        let arguments = GeneratedContent::from_json_str(&arguments_json)?;
        let output = registry.invoke(&tool_name, arguments)?;
        output.to_bridge_json()
    }));

    match result {
        Ok(Ok(output_json)) => {
            *output_json_out = swift_dup_string(&output_json);
            ffi::status::OK
        }
        Ok(Err(error)) => {
            *error_out = swift_dup_string(error.message());
            error.code()
        }
        Err(_) => {
            *error_out = swift_dup_string("tool callback panicked");
            ffi::status::TOOL_CALL_FAILED
        }
    }
}
