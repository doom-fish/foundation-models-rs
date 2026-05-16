#![doc = include_str!("../README.md")]
//!
//! ---
//!
//! # API Documentation
//!
//! Safe, idiomatic Rust bindings for Apple's [FoundationModels] framework —
//! the on-device large language model that ships with Apple Intelligence.
//!
//! Generate text, hold multi-turn conversations, and stream tokens from the
//! system language model on macOS 26.0+.
//!
//! [FoundationModels]: https://developer.apple.com/documentation/foundationmodels
//!
//! # Quick start
//!
//! ```rust,no_run
//! use foundation_models::{LanguageModelSession, SystemLanguageModel};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! if !SystemLanguageModel::is_available() {
//!     eprintln!("Model unavailable: {:?}", SystemLanguageModel::availability());
//!     return Ok(());
//! }
//!
//! let session = LanguageModelSession::new();
//! let reply = session.respond("Name three Norse gods.")?;
//! println!("{reply}");
//! # Ok(())
//! # }
//! ```
//!
//! # Streaming
//!
//! ```rust,no_run
//! use foundation_models::{LanguageModelSession, StreamEvent};
//! use std::io::Write;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let session = LanguageModelSession::new();
//! session.stream("Tell me a haiku about Rust.", |event| match event {
//!     StreamEvent::Chunk(s) => {
//!         print!("{s}");
//!         std::io::stdout().flush().ok();
//!     }
//!     StreamEvent::Done => println!(),
//!     StreamEvent::Error(e) => eprintln!("\nstream error: {e}"),
//!     _ => {}
//! })?;
//! # Ok(())
//! # }
//! ```

#![cfg_attr(docsrs, feature(doc_cfg))]
#![allow(
    clippy::derive_partial_eq_without_eq,
    clippy::doc_markdown,
    clippy::map_unwrap_or,
    clippy::missing_const_for_fn,
    clippy::missing_errors_doc,
    clippy::missing_fields_in_debug,
    clippy::missing_panics_doc,
    clippy::needless_pass_by_value,
    clippy::new_without_default,
    clippy::option_if_let_else,
    clippy::ref_option,
    clippy::semicolon_if_nothing_returned,
    clippy::significant_drop_in_scrutinee,
    clippy::struct_field_names,
    clippy::unnecessary_map_or,
    clippy::use_self
)]

pub mod content;
pub mod error;
pub mod ffi;
pub mod generation;
pub mod model;
pub mod prompt;
pub mod schema;
pub mod session;
pub mod tool;
pub mod transcript;

pub use content::{
    Decimal, FromGeneratedContent, GeneratedContent, GeneratedContentKind, GenerationId,
    ToGeneratedContent,
};
pub use error::{
    FMError, GenerationErrorContext, Refusal, SchemaErrorContext, ToolCallError, Unavailability,
};
pub use generation::{GenerationOptions, SamplingMode};
pub use model::{
    Adapter, Availability, ConfiguredSystemLanguageModel, Guardrails, SystemLanguageModel, UseCase,
};
pub use prompt::{
    Instructions, Prompt, ResponseFormat, Segment, StructuredSegment, TextSegment, ToInstructions,
    ToPrompt, ToolDefinition,
};
pub use schema::{
    DynamicGenerationProperty, DynamicGenerationSchema, Generable, GenerationGuide,
    GenerationSchema,
};
pub use session::{
    FeedbackAttachmentRequest, FeedbackIssue, FeedbackIssueCategory, FeedbackSentiment,
    LanguageModelSession, SessionBuilder, SessionResponse, StreamEvent, StructuredStreamEvent,
    StructuredStreamSnapshot,
};
pub use tool::{Tool, ToolOutput, ToolSpec};
pub use transcript::{
    Entry as TranscriptEntry, ToolCall, ToolCalls, ToolOutput as TranscriptToolOutput, Transcript,
    TranscriptInstructions, TranscriptPrompt, TranscriptResponse,
};

/// Common imports for users of this crate.
pub mod prelude {
    pub use crate::content::{
        Decimal, FromGeneratedContent, GeneratedContent, GeneratedContentKind, GenerationId,
        ToGeneratedContent,
    };
    pub use crate::error::{
        FMError, GenerationErrorContext, Refusal, SchemaErrorContext, ToolCallError, Unavailability,
    };
    pub use crate::generation::{GenerationOptions, SamplingMode};
    pub use crate::model::{
        Adapter, Availability, ConfiguredSystemLanguageModel, Guardrails, SystemLanguageModel,
        UseCase,
    };
    pub use crate::prompt::{
        Instructions, Prompt, ResponseFormat, Segment, StructuredSegment, TextSegment,
        ToInstructions, ToPrompt, ToolDefinition,
    };
    pub use crate::schema::{
        DynamicGenerationProperty, DynamicGenerationSchema, Generable, GenerationGuide,
        GenerationSchema,
    };
    pub use crate::session::{
        FeedbackAttachmentRequest, FeedbackIssue, FeedbackIssueCategory, FeedbackSentiment,
        LanguageModelSession, SessionBuilder, SessionResponse, StreamEvent, StructuredStreamEvent,
        StructuredStreamSnapshot,
    };
    pub use crate::tool::{Tool, ToolOutput, ToolSpec};
    pub use crate::transcript::{
        Entry as TranscriptEntry, ToolCall, ToolCalls, ToolOutput as TranscriptToolOutput,
        Transcript, TranscriptInstructions, TranscriptPrompt, TranscriptResponse,
    };
}
