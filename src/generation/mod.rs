//! Knobs that control how the model produces text.

use crate::ffi;

/// Strategy used when sampling the next token.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
#[derive(Default)]
pub enum SamplingMode {
    /// Defer to `FoundationModels`' default sampling strategy.
    #[default]
    Default,
    /// Always pick the highest-probability token. Deterministic.
    Greedy,
    /// Sample from the top-`k` most probable tokens.
    TopK(u32),
    /// Nucleus sampling: smallest set of tokens whose cumulative probability
    /// exceeds `p` (must be in `0.0..=1.0`).
    TopP(f64),
}

/// Generation knobs. All fields are optional; unset fields keep the
/// model's defaults.
///
/// # Examples
///
/// ```rust
/// use foundation_models::{GenerationOptions, SamplingMode};
///
/// let opts = GenerationOptions::new()
///     .with_temperature(0.7)
///     .with_maximum_response_tokens(500)
///     .with_sampling(SamplingMode::TopP(0.9));
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct GenerationOptions {
    temperature: Option<f64>,
    max_tokens: Option<u32>,
    sampling: SamplingMode,
}

impl GenerationOptions {
    /// Create options with all fields set to their defaults.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            temperature: None,
            max_tokens: None,
            sampling: SamplingMode::Default,
        }
    }

    /// Sampling temperature; higher values produce more varied output.
    /// `FoundationModels` accepts values in `0.0..=2.0`.
    #[must_use]
    pub const fn with_temperature(mut self, temperature: f64) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Hard cap on the number of tokens the model may emit.
    #[must_use]
    pub const fn with_maximum_response_tokens(mut self, tokens: u32) -> Self {
        self.max_tokens = Some(tokens);
        self
    }

    /// Override the sampling strategy.
    #[must_use]
    pub const fn with_sampling(mut self, mode: SamplingMode) -> Self {
        self.sampling = mode;
        self
    }

    /// Lower into the C-compatible struct shared with Swift.
    pub(crate) fn to_ffi(self) -> ffi::FFIGenerationOptions {
        let (mode_code, top_k, top_p) = match self.sampling {
            SamplingMode::Default => (0, 0, 0.0),
            SamplingMode::Greedy => (1, 0, 0.0),
            SamplingMode::TopK(k) => (2, i32::try_from(k).unwrap_or(i32::MAX), 0.0),
            SamplingMode::TopP(p) => (3, 0, p),
        };
        ffi::FFIGenerationOptions {
            temperature: self.temperature.unwrap_or(f64::NAN),
            maximum_response_tokens: self
                .max_tokens
                .map_or(0, |t| i32::try_from(t).unwrap_or(i32::MAX)),
            sampling_mode: mode_code,
            top_k,
            top_p,
        }
    }
}
