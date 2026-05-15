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

pub mod error;
pub mod ffi;
pub mod generation;
pub mod model;
pub mod session;

pub use error::{FMError, Unavailability};
pub use generation::{GenerationOptions, SamplingMode};
pub use model::{Availability, SystemLanguageModel};
pub use session::{LanguageModelSession, StreamEvent};

/// Common imports for users of this crate.
pub mod prelude {
    pub use crate::error::{FMError, Unavailability};
    pub use crate::generation::{GenerationOptions, SamplingMode};
    pub use crate::model::{Availability, SystemLanguageModel};
    pub use crate::session::{LanguageModelSession, StreamEvent};
}
