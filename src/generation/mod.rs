//! Knobs that control how the model produces text.

use serde_json::{Map, Value};

use crate::ffi;

/// Strategy used when sampling the next token.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[non_exhaustive]
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

/// Generation knobs. All fields are optional; unset fields keep the model's
/// defaults.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct GenerationOptions {
    temperature: Option<f64>,
    max_tokens: Option<u32>,
    sampling: SamplingMode,
    sampling_seed: Option<u64>,
}

impl GenerationOptions {
    /// Create options with all fields set to their defaults.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            temperature: None,
            max_tokens: None,
            sampling: SamplingMode::Default,
            sampling_seed: None,
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

    /// Use a deterministic random seed for non-greedy sampling.
    #[must_use]
    pub const fn with_sampling_seed(mut self, seed: u64) -> Self {
        self.sampling_seed = Some(seed);
        self
    }

    /// Sampling temperature, if explicitly set.
    #[must_use]
    pub const fn temperature(self) -> Option<f64> {
        self.temperature
    }

    /// The explicit token cap, if any.
    #[must_use]
    pub const fn maximum_response_tokens(self) -> Option<u32> {
        self.max_tokens
    }

    /// The configured sampling strategy.
    #[must_use]
    pub const fn sampling(self) -> SamplingMode {
        self.sampling
    }

    /// The deterministic random seed for top-k / top-p sampling.
    #[must_use]
    pub const fn sampling_seed(self) -> Option<u64> {
        self.sampling_seed
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
                .map_or(0, |tokens| i32::try_from(tokens).unwrap_or(i32::MAX)),
            sampling_mode: mode_code,
            top_k,
            top_p,
            random_seed: self.sampling_seed.unwrap_or(0),
            has_random_seed: self.sampling_seed.is_some(),
        }
    }

    pub(crate) fn to_transcript_json_value(self) -> Value {
        let mut map = Map::new();
        if let Some(temperature) = self.temperature {
            map.insert("temperature".into(), Value::from(temperature));
        }
        if let Some(max_tokens) = self.max_tokens {
            map.insert("maximumResponseTokens".into(), Value::from(max_tokens));
        }
        if let Some(seed) = self.sampling_seed {
            map.insert("randomSeed".into(), Value::from(seed));
        }
        match self.sampling {
            SamplingMode::Default | SamplingMode::Greedy => {}
            SamplingMode::TopK(k) => {
                map.insert("topK".into(), Value::from(k));
            }
            SamplingMode::TopP(p) => {
                map.insert("topP".into(), Value::from(p));
            }
        }
        Value::Object(map)
    }

    #[must_use]
    pub(crate) fn from_transcript_json_value(value: Option<&Value>) -> Self {
        let Some(Value::Object(map)) = value else {
            return Self::new();
        };
        let sampling = if let Some(top_k) = map.get("topK").and_then(Value::as_u64) {
            SamplingMode::TopK(u32::try_from(top_k).unwrap_or(u32::MAX))
        } else if let Some(top_p) = map.get("topP").and_then(Value::as_f64) {
            SamplingMode::TopP(top_p)
        } else {
            SamplingMode::Default
        };
        Self {
            temperature: map.get("temperature").and_then(Value::as_f64),
            max_tokens: map
                .get("maximumResponseTokens")
                .and_then(Value::as_u64)
                .and_then(|tokens| u32::try_from(tokens).ok()),
            sampling,
            sampling_seed: map.get("randomSeed").and_then(Value::as_u64),
        }
    }
}
