//! Prompt and instructions builders.

use serde_json::{json, Value};

use crate::content::GeneratedContent;
use crate::error::FMError;
use crate::schema::GenerationSchema;

/// A FoundationModels prompt.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Prompt {
    segments: Vec<Segment>,
}

impl Prompt {
    /// Create an empty prompt.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            segments: Vec::new(),
        }
    }

    /// Create a prompt from a single text segment.
    #[must_use]
    pub fn text(text: impl Into<String>) -> Self {
        Self::from(text.into())
    }

    /// Create a prompt from a structured content segment.
    #[must_use]
    pub fn structured(content: GeneratedContent) -> Self {
        Self::from(content)
    }

    /// Append a text segment.
    pub fn push_text(&mut self, text: impl Into<String>) {
        self.segments.push(Segment::text(text));
    }

    /// Append a structured content segment.
    pub fn push_structured(&mut self, source: impl Into<String>, content: GeneratedContent) {
        self.segments.push(Segment::structure(source, content));
    }

    /// Borrow the prompt segments.
    #[must_use]
    pub fn segments(&self) -> &[Segment] {
        &self.segments
    }

    /// Consume the prompt and return its segments.
    #[must_use]
    pub fn into_segments(self) -> Vec<Segment> {
        self.segments
    }

    pub(crate) fn to_bridge_value(&self) -> Value {
        json!({
            "segments": self.segments.iter().map(Segment::to_bridge_value).collect::<Vec<_>>()
        })
    }

    pub(crate) fn to_bridge_json(&self) -> Result<String, FMError> {
        serde_json::to_string(&self.to_bridge_value()).map_err(|error| {
            FMError::InvalidArgument(format!("prompt is not JSON-serializable: {error}"))
        })
    }
}

impl From<String> for Prompt {
    fn from(text: String) -> Self {
        Self {
            segments: vec![Segment::text(text)],
        }
    }
}

impl From<&str> for Prompt {
    fn from(text: &str) -> Self {
        Self::from(text.to_owned())
    }
}

impl From<GeneratedContent> for Prompt {
    fn from(content: GeneratedContent) -> Self {
        Self {
            segments: vec![Segment::structure("GeneratedContent", content)],
        }
    }
}

impl From<Vec<Segment>> for Prompt {
    fn from(segments: Vec<Segment>) -> Self {
        Self { segments }
    }
}

/// A FoundationModels instructions value.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Instructions {
    segments: Vec<Segment>,
}

impl Instructions {
    /// Create empty instructions.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            segments: Vec::new(),
        }
    }

    /// Append a text segment.
    pub fn push_text(&mut self, text: impl Into<String>) {
        self.segments.push(Segment::text(text));
    }

    /// Append a structured content segment.
    pub fn push_structured(&mut self, source: impl Into<String>, content: GeneratedContent) {
        self.segments.push(Segment::structure(source, content));
    }

    /// Borrow the instruction segments.
    #[must_use]
    pub fn segments(&self) -> &[Segment] {
        &self.segments
    }

    /// Consume the instructions and return their segments.
    #[must_use]
    pub fn into_segments(self) -> Vec<Segment> {
        self.segments
    }

    pub(crate) fn to_bridge_value(&self) -> Value {
        json!({
            "segments": self.segments.iter().map(Segment::to_bridge_value).collect::<Vec<_>>()
        })
    }

    pub(crate) fn to_bridge_json(&self) -> Result<String, FMError> {
        serde_json::to_string(&self.to_bridge_value()).map_err(|error| {
            FMError::InvalidArgument(format!("instructions are not JSON-serializable: {error}"))
        })
    }
}

impl From<String> for Instructions {
    fn from(text: String) -> Self {
        Self {
            segments: vec![Segment::text(text)],
        }
    }
}

impl From<&str> for Instructions {
    fn from(text: &str) -> Self {
        Self::from(text.to_owned())
    }
}

impl From<GeneratedContent> for Instructions {
    fn from(content: GeneratedContent) -> Self {
        Self {
            segments: vec![Segment::structure("GeneratedContent", content)],
        }
    }
}

impl From<Vec<Segment>> for Instructions {
    fn from(segments: Vec<Segment>) -> Self {
        Self { segments }
    }
}

/// A prompt or transcript segment.
#[derive(Debug, Clone, PartialEq)]
pub enum Segment {
    Text(TextSegment),
    Structure(StructuredSegment),
}

impl Segment {
    /// Create a text segment.
    #[must_use]
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text(TextSegment {
            id: None,
            text: text.into(),
        })
    }

    /// Create a structured segment.
    #[must_use]
    pub fn structure(source: impl Into<String>, content: GeneratedContent) -> Self {
        Self::Structure(StructuredSegment {
            id: None,
            source: source.into(),
            content,
        })
    }

    pub(crate) fn to_bridge_value(&self) -> Value {
        match self {
            Self::Text(segment) => json!({
                "kind": "text",
                "text": segment.text,
            }),
            Self::Structure(segment) => json!({
                "kind": "structure",
                "source": segment.source,
                "contentJSON": segment.content.json_string().expect("generated content must serialize")
            }),
        }
    }
}

/// A plain-text transcript segment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextSegment {
    pub id: Option<String>,
    pub text: String,
}

/// A structured transcript segment.
#[derive(Debug, Clone, PartialEq)]
pub struct StructuredSegment {
    pub id: Option<String>,
    pub source: String,
    pub content: GeneratedContent,
}

/// A transcript response format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResponseFormat {
    name: Option<String>,
    schema: GenerationSchema,
}

impl ResponseFormat {
    /// Create a response format from a generation schema.
    #[must_use]
    pub fn json_schema(schema: GenerationSchema) -> Self {
        Self { name: None, schema }
    }

    pub(crate) fn from_transcript_json_value(value: &Value) -> Result<Self, FMError> {
        let schema = value
            .get("jsonSchema")
            .and_then(|json_schema| json_schema.get("schema"))
            .ok_or_else(|| {
                FMError::DecodingFailure("response format is missing jsonSchema.schema".into())
            })?;
        let name = value
            .get("jsonSchema")
            .and_then(|json_schema| json_schema.get("name"))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        Ok(Self {
            name,
            schema: GenerationSchema::from_json_schema_unchecked(
                serde_json::to_string(schema).map_err(|error| {
                    FMError::InvalidArgument(format!(
                        "response format schema is not valid JSON: {error}"
                    ))
                })?,
            ),
        })
    }

    /// Attach an explicit display name.
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// The underlying schema.
    #[must_use]
    pub const fn schema(&self) -> &GenerationSchema {
        &self.schema
    }

    pub(crate) fn to_transcript_json_value(&self) -> Value {
        let schema_value: Value = serde_json::from_str(self.schema.json_schema())
            .expect("validated generation schema must always be valid JSON");
        json!({
            "type": "jsonSchema",
            "jsonSchema": {
                "name": self
                    .name
                    .clone()
                    .or_else(|| self.schema.name())
                    .unwrap_or_else(|| "GeneratedContent".to_string()),
                "schema": schema_value,
            }
        })
    }
}

/// A transcript tool definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: GenerationSchema,
}

impl ToolDefinition {
    /// Create a tool definition.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        parameters: GenerationSchema,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters,
        }
    }

    pub(crate) fn to_transcript_json_value(&self) -> Value {
        let parameters: Value = serde_json::from_str(self.parameters.json_schema())
            .expect("validated generation schema must always be valid JSON");
        json!({
            "type": "function",
            "function": {
                "name": self.name,
                "description": self.description,
                "parameters": parameters,
            }
        })
    }
}

/// Convert a Rust value into a FoundationModels prompt.
pub trait ToPrompt {
    /// Convert the value into a prompt.
    fn to_prompt(self) -> Result<Prompt, FMError>;
}

impl ToPrompt for Prompt {
    fn to_prompt(self) -> Result<Prompt, FMError> {
        Ok(self)
    }
}

impl ToPrompt for &Prompt {
    fn to_prompt(self) -> Result<Prompt, FMError> {
        Ok(self.clone())
    }
}

impl ToPrompt for String {
    fn to_prompt(self) -> Result<Prompt, FMError> {
        Ok(Prompt::from(self))
    }
}

impl ToPrompt for &str {
    fn to_prompt(self) -> Result<Prompt, FMError> {
        Ok(Prompt::from(self))
    }
}

impl ToPrompt for GeneratedContent {
    fn to_prompt(self) -> Result<Prompt, FMError> {
        Ok(Prompt::from(self))
    }
}

impl ToPrompt for &GeneratedContent {
    fn to_prompt(self) -> Result<Prompt, FMError> {
        Ok(Prompt::from(self.clone()))
    }
}

/// Convert a Rust value into FoundationModels instructions.
pub trait ToInstructions {
    /// Convert the value into instructions.
    fn to_instructions(self) -> Result<Instructions, FMError>;
}

impl ToInstructions for Instructions {
    fn to_instructions(self) -> Result<Instructions, FMError> {
        Ok(self)
    }
}

impl ToInstructions for &Instructions {
    fn to_instructions(self) -> Result<Instructions, FMError> {
        Ok(self.clone())
    }
}

impl ToInstructions for String {
    fn to_instructions(self) -> Result<Instructions, FMError> {
        Ok(Instructions::from(self))
    }
}

impl ToInstructions for &str {
    fn to_instructions(self) -> Result<Instructions, FMError> {
        Ok(Instructions::from(self))
    }
}

impl ToInstructions for GeneratedContent {
    fn to_instructions(self) -> Result<Instructions, FMError> {
        Ok(Instructions::from(self))
    }
}

impl ToInstructions for &GeneratedContent {
    fn to_instructions(self) -> Result<Instructions, FMError> {
        Ok(Instructions::from(self.clone()))
    }
}
