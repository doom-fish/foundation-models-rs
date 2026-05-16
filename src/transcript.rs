//! Transcript inspection and restoration.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Map, Value};

use crate::content::GeneratedContent;
use crate::error::FMError;
use crate::generation::GenerationOptions;
use crate::prompt::{
    Instructions, ResponseFormat, Segment, StructuredSegment, TextSegment, ToolDefinition,
};

static NEXT_SYNTHETIC_ID: AtomicU64 = AtomicU64::new(1);

fn synthetic_id(prefix: &str) -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let counter = NEXT_SYNTHETIC_ID.fetch_add(1, Ordering::Relaxed);
    format!("{prefix}-{millis}-{counter}")
}

/// A session transcript.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Transcript {
    entries: Vec<Entry>,
}

impl Transcript {
    /// Create an empty transcript.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Create a transcript from entries.
    #[must_use]
    pub fn from_entries(entries: Vec<Entry>) -> Self {
        Self { entries }
    }

    /// Borrow the transcript entries.
    #[must_use]
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// Push a transcript entry.
    pub fn push(&mut self, entry: Entry) {
        self.entries.push(entry);
    }

    /// Parse a FoundationModels transcript JSON string.
    ///
    /// # Errors
    ///
    /// Returns [`FMError::DecodingFailure`] if `json` does not match the SDK's
    /// transcript encoding.
    pub fn from_json_str(json: &str) -> Result<Self, FMError> {
        let root: Value = serde_json::from_str(json)
            .map_err(|error| FMError::DecodingFailure(error.to_string()))?;
        let entries = root
            .get("transcript")
            .and_then(|transcript| transcript.get("entries"))
            .and_then(Value::as_array)
            .ok_or_else(|| {
                FMError::DecodingFailure("transcript JSON is missing transcript.entries".into())
            })?;
        let entries = entries
            .iter()
            .map(Entry::from_json_value)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { entries })
    }

    /// Serialize the transcript back to FoundationModels' native JSON shape.
    ///
    /// # Errors
    ///
    /// Returns [`FMError::InvalidArgument`] if one of the entries contains an
    /// invalid JSON payload.
    pub fn to_json_string(&self) -> Result<String, FMError> {
        serde_json::to_string(&json!({
            "version": 1,
            "type": "FoundationModels.Transcript",
            "transcript": {
                "entries": self.entries.iter().map(Entry::to_json_value).collect::<Result<Vec<_>, _>>()?
            }
        }))
        .map_err(|error| FMError::InvalidArgument(format!("failed to encode transcript JSON: {error}")))
    }
}

impl From<Vec<Entry>> for Transcript {
    fn from(entries: Vec<Entry>) -> Self {
        Self::from_entries(entries)
    }
}

/// One transcript entry.
#[derive(Debug, Clone, PartialEq)]
pub enum Entry {
    Instructions(TranscriptInstructions),
    Prompt(TranscriptPrompt),
    ToolCalls(ToolCalls),
    ToolOutput(ToolOutput),
    Response(TranscriptResponse),
}

impl Entry {
    fn from_json_value(value: &Value) -> Result<Self, FMError> {
        let role = value
            .get("role")
            .and_then(Value::as_str)
            .ok_or_else(|| FMError::DecodingFailure("transcript entry is missing role".into()))?;
        match role {
            "instructions" => Ok(Self::Instructions(TranscriptInstructions::from_json_value(
                value,
            )?)),
            "user" => Ok(Self::Prompt(TranscriptPrompt::from_json_value(value)?)),
            "tool" => Ok(Self::ToolOutput(ToolOutput::from_json_value(value)?)),
            "response" if value.get("toolCalls").is_some() => {
                Ok(Self::ToolCalls(ToolCalls::from_json_value(value)?))
            }
            "response" => Ok(Self::Response(TranscriptResponse::from_json_value(value)?)),
            other => Err(FMError::DecodingFailure(format!(
                "unsupported transcript role `{other}`"
            ))),
        }
    }

    fn to_json_value(&self) -> Result<Value, FMError> {
        match self {
            Self::Instructions(entry) => entry.to_json_value(),
            Self::Prompt(entry) => entry.to_json_value(),
            Self::ToolCalls(entry) => entry.to_json_value(),
            Self::ToolOutput(entry) => entry.to_json_value(),
            Self::Response(entry) => entry.to_json_value(),
        }
    }
}

/// An instructions transcript entry.
#[derive(Debug, Clone, PartialEq)]
pub struct TranscriptInstructions {
    pub id: Option<String>,
    pub instructions: Instructions,
    pub tool_definitions: Vec<ToolDefinition>,
}

impl TranscriptInstructions {
    fn from_json_value(value: &Value) -> Result<Self, FMError> {
        Ok(Self {
            id: value
                .get("id")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            instructions: Instructions::from(parse_segments(value.get("contents"))?),
            tool_definitions: parse_tool_definitions(value.get("tools"))?,
        })
    }

    fn to_json_value(&self) -> Result<Value, FMError> {
        let mut object = Map::new();
        object.insert("role".into(), Value::String("instructions".into()));
        object.insert(
            "id".into(),
            Value::String(
                self.id
                    .clone()
                    .unwrap_or_else(|| synthetic_id("instructions")),
            ),
        );
        object.insert(
            "contents".into(),
            segments_to_json(self.instructions.segments())?,
        );
        if !self.tool_definitions.is_empty() {
            object.insert(
                "tools".into(),
                Value::Array(
                    self.tool_definitions
                        .iter()
                        .map(ToolDefinition::to_transcript_json_value)
                        .collect(),
                ),
            );
        }
        Ok(Value::Object(object))
    }
}

/// A user-prompt transcript entry.
#[derive(Debug, Clone, PartialEq)]
pub struct TranscriptPrompt {
    pub id: Option<String>,
    pub prompt: crate::prompt::Prompt,
    pub options: GenerationOptions,
    pub response_format: Option<ResponseFormat>,
}

impl TranscriptPrompt {
    fn from_json_value(value: &Value) -> Result<Self, FMError> {
        Ok(Self {
            id: value
                .get("id")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            prompt: crate::prompt::Prompt::from(parse_segments(value.get("contents"))?),
            options: GenerationOptions::from_transcript_json_value(value.get("options")),
            response_format: value
                .get("responseFormat")
                .map(ResponseFormat::from_transcript_json_value)
                .transpose()?,
        })
    }

    fn to_json_value(&self) -> Result<Value, FMError> {
        let mut object = Map::new();
        object.insert("role".into(), Value::String("user".into()));
        object.insert(
            "id".into(),
            Value::String(self.id.clone().unwrap_or_else(|| synthetic_id("prompt"))),
        );
        object.insert("contents".into(), segments_to_json(self.prompt.segments())?);
        object.insert("options".into(), self.options.to_transcript_json_value());
        if let Some(response_format) = &self.response_format {
            object.insert(
                "responseFormat".into(),
                response_format.to_transcript_json_value(),
            );
        }
        Ok(Value::Object(object))
    }
}

/// A transcript entry that records tool calls the model made.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolCalls {
    pub id: Option<String>,
    pub calls: Vec<ToolCall>,
}

impl ToolCalls {
    fn from_json_value(value: &Value) -> Result<Self, FMError> {
        Ok(Self {
            id: value
                .get("id")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            calls: value
                .get("toolCalls")
                .and_then(Value::as_array)
                .map_or(&[] as &[Value], Vec::as_slice)
                .iter()
                .map(ToolCall::from_json_value)
                .collect::<Result<Vec<_>, _>>()?,
        })
    }

    fn to_json_value(&self) -> Result<Value, FMError> {
        Ok(json!({
            "role": "response",
            "id": self.id.clone().unwrap_or_else(|| synthetic_id("tool-calls")),
            "toolCalls": self.calls.iter().map(ToolCall::to_json_value).collect::<Result<Vec<_>, _>>()?,
        }))
    }
}

/// One tool call entry.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolCall {
    pub id: String,
    pub tool_name: String,
    pub arguments: GeneratedContent,
}

impl ToolCall {
    fn from_json_value(value: &Value) -> Result<Self, FMError> {
        let arguments = value
            .get("arguments")
            .and_then(Value::as_str)
            .ok_or_else(|| FMError::DecodingFailure("tool call is missing arguments".into()))?;
        Ok(Self {
            id: value
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            tool_name: value
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            arguments: GeneratedContent::from_json_str(arguments)?,
        })
    }

    fn to_json_value(&self) -> Result<Value, FMError> {
        Ok(json!({
            "id": self.id,
            "name": self.tool_name,
            "arguments": self.arguments.json_string()?,
        }))
    }
}

/// A tool output transcript entry.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolOutput {
    pub id: String,
    pub tool_name: String,
    pub tool_call_id: Option<String>,
    pub segments: Vec<Segment>,
}

impl ToolOutput {
    fn from_json_value(value: &Value) -> Result<Self, FMError> {
        Ok(Self {
            id: value
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            tool_name: value
                .get("toolName")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            tool_call_id: value
                .get("toolCallID")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            segments: parse_segments(value.get("contents"))?,
        })
    }

    fn to_json_value(&self) -> Result<Value, FMError> {
        Ok(json!({
            "role": "tool",
            "id": self.id,
            "toolCallID": self.tool_call_id.clone().unwrap_or_else(|| self.id.clone()),
            "toolName": self.tool_name,
            "contents": segments_to_json(&self.segments)?,
        }))
    }
}

/// A model response transcript entry.
#[derive(Debug, Clone, PartialEq)]
pub struct TranscriptResponse {
    pub id: Option<String>,
    pub asset_ids: Vec<String>,
    pub segments: Vec<Segment>,
}

impl TranscriptResponse {
    fn from_json_value(value: &Value) -> Result<Self, FMError> {
        Ok(Self {
            id: value
                .get("id")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            asset_ids: value
                .get("assets")
                .and_then(Value::as_array)
                .map(|assets| {
                    assets
                        .iter()
                        .filter_map(Value::as_str)
                        .map(ToOwned::to_owned)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default(),
            segments: parse_segments(value.get("contents"))?,
        })
    }

    fn to_json_value(&self) -> Result<Value, FMError> {
        Ok(json!({
            "role": "response",
            "id": self.id.clone().unwrap_or_else(|| synthetic_id("response")),
            "assets": self.asset_ids,
            "contents": segments_to_json(&self.segments)?,
        }))
    }
}

fn parse_segments(value: Option<&Value>) -> Result<Vec<Segment>, FMError> {
    value
        .and_then(Value::as_array)
        .map_or(&[] as &[Value], Vec::as_slice)
        .iter()
        .map(|segment| {
            let segment_type = segment
                .get("type")
                .and_then(Value::as_str)
                .ok_or_else(|| FMError::DecodingFailure("segment is missing type".into()))?;
            match segment_type {
                "text" => Ok(Segment::Text(TextSegment {
                    id: segment
                        .get("id")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned),
                    text: segment
                        .get("text")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                })),
                "structure" => {
                    let structure = segment.get("structure").ok_or_else(|| {
                        FMError::DecodingFailure("structured segment is missing structure".into())
                    })?;
                    let content = structure.get("content").ok_or_else(|| {
                        FMError::DecodingFailure("structured segment is missing content".into())
                    })?;
                    Ok(Segment::Structure(StructuredSegment {
                        id: segment
                            .get("id")
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned),
                        source: structure
                            .get("source")
                            .and_then(Value::as_str)
                            .unwrap_or("GeneratedContent")
                            .to_string(),
                        content: GeneratedContent::from_json_str(
                            &serde_json::to_string(content).map_err(|error| {
                                FMError::InvalidArgument(format!(
                                    "structured segment content is not valid JSON: {error}"
                                ))
                            })?,
                        )?,
                    }))
                }
                other => Err(FMError::DecodingFailure(format!(
                    "unsupported segment type `{other}`"
                ))),
            }
        })
        .collect()
}

fn segments_to_json(segments: &[Segment]) -> Result<Value, FMError> {
    Ok(Value::Array(
        segments
            .iter()
            .map(|segment| match segment {
                Segment::Text(TextSegment { id, text }) => Ok(json!({
                    "type": "text",
                    "id": id.clone().unwrap_or_else(|| synthetic_id("segment-text")),
                    "text": text,
                })),
                Segment::Structure(StructuredSegment {
                    id,
                    source,
                    content,
                }) => {
                    let content_value: Value = serde_json::from_str(&content.json_string()?)
                        .map_err(|error| {
                            FMError::InvalidArgument(format!(
                                "structured segment content is not valid JSON: {error}"
                            ))
                        })?;
                    Ok(json!({
                        "type": "structure",
                        "id": id.clone().unwrap_or_else(|| synthetic_id("segment-structure")),
                        "structure": {
                            "source": source,
                            "content": content_value,
                        }
                    }))
                }
            })
            .collect::<Result<Vec<_>, _>>()?,
    ))
}

fn parse_tool_definitions(value: Option<&Value>) -> Result<Vec<ToolDefinition>, FMError> {
    value
        .and_then(Value::as_array)
        .map_or(&[] as &[Value], Vec::as_slice)
        .iter()
        .map(|tool| {
            let function = tool.get("function").ok_or_else(|| {
                FMError::DecodingFailure("tool definition is missing function body".into())
            })?;
            let parameters = function.get("parameters").ok_or_else(|| {
                FMError::DecodingFailure("tool definition is missing parameters".into())
            })?;
            Ok(ToolDefinition::new(
                function
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
                function
                    .get("description")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
                crate::schema::GenerationSchema::from_json_schema_unchecked(
                    serde_json::to_string(parameters).map_err(|error| {
                        FMError::InvalidArgument(format!(
                            "tool parameters are not valid JSON: {error}"
                        ))
                    })?,
                ),
            ))
        })
        .collect()
}
