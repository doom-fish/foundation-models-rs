//! [`GeneratedContent`] — structured model output represented as JSON.

use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

use crate::error::FMError;

/// Rust analogue of FoundationModels' `ConvertibleFromGeneratedContent`.
pub trait FromGeneratedContent: Sized {
    /// Decode a Rust value from generated content.
    fn from_generated_content(content: &GeneratedContent) -> Result<Self, FMError>;
}

/// Rust analogue of FoundationModels' `ConvertibleToGeneratedContent`.
pub trait ToGeneratedContent {
    /// Convert a Rust value into generated content.
    fn to_generated_content(&self) -> Result<GeneratedContent, FMError>;
}

/// A piece of generated structured content.
///
/// Apple models structured generations as `GeneratedContent`; the Rust wrapper
/// stores the JSON value plus the metadata that Apple's streaming API exposes.
#[derive(Debug, Clone, PartialEq)]
pub struct GeneratedContent {
    value: Value,
    generation_id: Option<String>,
    is_complete: bool,
}

impl GeneratedContent {
    /// Parse a JSON string into generated content.
    ///
    /// # Errors
    ///
    /// Returns [`FMError::InvalidArgument`] if `json` is not valid JSON.
    pub fn from_json_str(json: &str) -> Result<Self, FMError> {
        let value = serde_json::from_str(json).map_err(|error| {
            FMError::InvalidArgument(format!("generated content JSON is invalid: {error}"))
        })?;
        Ok(Self {
            value,
            generation_id: None,
            is_complete: true,
        })
    }

    /// Convert a serializable Rust value into generated content.
    ///
    /// # Errors
    ///
    /// Returns [`FMError::InvalidArgument`] if `value` cannot be encoded as JSON.
    pub fn from_value<T>(value: T) -> Result<Self, FMError>
    where
        T: Serialize,
    {
        let value = serde_json::to_value(value).map_err(|error| {
            FMError::InvalidArgument(format!(
                "generated content value is not JSON-serializable: {error}"
            ))
        })?;
        Ok(Self {
            value,
            generation_id: None,
            is_complete: true,
        })
    }

    /// Build a value from bridge metadata.
    pub(crate) fn from_bridge_json(
        json: &str,
        is_complete: bool,
        generation_id: Option<String>,
    ) -> Result<Self, FMError> {
        let mut content = Self::from_json_str(json)?;
        content.is_complete = is_complete;
        content.generation_id = generation_id;
        Ok(content)
    }

    /// Return the underlying JSON value.
    #[must_use]
    pub const fn raw_value(&self) -> &Value {
        &self.value
    }

    /// Consume the content and return the underlying JSON value.
    #[must_use]
    pub fn into_raw_value(self) -> Value {
        self.value
    }

    /// Serialize the content back to a compact JSON string.
    ///
    /// # Errors
    ///
    /// Returns [`FMError::Unknown`] if serialization fails.
    pub fn json_string(&self) -> Result<String, FMError> {
        serde_json::to_string(&self.value).map_err(|error| FMError::Unknown {
            code: crate::ffi::status::UNKNOWN,
            message: format!("failed to serialize generated content: {error}"),
        })
    }

    /// Serialize the content as pretty JSON.
    ///
    /// # Errors
    ///
    /// Returns [`FMError::Unknown`] if serialization fails.
    pub fn json_string_pretty(&self) -> Result<String, FMError> {
        serde_json::to_string_pretty(&self.value).map_err(|error| FMError::Unknown {
            code: crate::ffi::status::UNKNOWN,
            message: format!("failed to serialize generated content: {error}"),
        })
    }

    /// Decode the content into a Rust value.
    ///
    /// # Errors
    ///
    /// Returns [`FMError::DecodingFailure`] if the JSON value does not match `T`.
    pub fn value<T>(&self) -> Result<T, FMError>
    where
        T: DeserializeOwned,
    {
        serde_json::from_value(self.value.clone())
            .map_err(|error| FMError::DecodingFailure(error.to_string()))
    }

    /// Decode a named property from an object content value.
    ///
    /// # Errors
    ///
    /// Returns [`FMError::DecodingFailure`] if the value is not an object, the
    /// property does not exist, or the property cannot be decoded as `T`.
    pub fn value_for_property<T>(&self, property: &str) -> Result<T, FMError>
    where
        T: DeserializeOwned,
    {
        let Value::Object(map) = &self.value else {
            return Err(FMError::DecodingFailure(
                "generated content is not an object".into(),
            ));
        };
        let value = map.get(property).cloned().ok_or_else(|| {
            FMError::DecodingFailure(format!(
                "generated content is missing property `{property}`"
            ))
        })?;
        serde_json::from_value(value).map_err(|error| FMError::DecodingFailure(error.to_string()))
    }

    /// Whether Apple's structured stream reported this content as complete.
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.is_complete
    }

    /// Apple's opaque generation identifier, if one was attached.
    #[must_use]
    pub fn generation_id(&self) -> Option<&str> {
        self.generation_id.as_deref()
    }
}

impl TryFrom<Value> for GeneratedContent {
    type Error = FMError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        Ok(Self {
            value,
            generation_id: None,
            is_complete: true,
        })
    }
}

impl From<GeneratedContent> for Value {
    fn from(value: GeneratedContent) -> Self {
        value.value
    }
}

macro_rules! impl_scalar_content {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl From<$ty> for GeneratedContent {
                fn from(value: $ty) -> Self {
                    Self {
                        value: serde_json::to_value(value)
                            .expect("scalar values must always be JSON-serializable"),
                        generation_id: None,
                        is_complete: true,
                    }
                }
            }
        )+
    };
}

impl_scalar_content!(bool, f32, f64, i8, i16, i32, i64, u8, u16, u32, u64);

impl From<String> for GeneratedContent {
    fn from(value: String) -> Self {
        Self {
            value: Value::String(value),
            generation_id: None,
            is_complete: true,
        }
    }
}

impl From<&str> for GeneratedContent {
    fn from(value: &str) -> Self {
        Self::from(value.to_owned())
    }
}

impl<T> From<Vec<T>> for GeneratedContent
where
    T: Into<GeneratedContent>,
{
    fn from(values: Vec<T>) -> Self {
        Self {
            value: Value::Array(values.into_iter().map(|value| value.into().value).collect()),
            generation_id: None,
            is_complete: true,
        }
    }
}

impl FromGeneratedContent for GeneratedContent {
    fn from_generated_content(content: &GeneratedContent) -> Result<Self, FMError> {
        Ok(content.clone())
    }
}

impl ToGeneratedContent for GeneratedContent {
    fn to_generated_content(&self) -> Result<GeneratedContent, FMError> {
        Ok(self.clone())
    }
}

impl FromGeneratedContent for Value {
    fn from_generated_content(content: &GeneratedContent) -> Result<Self, FMError> {
        Ok(content.raw_value().clone())
    }
}

impl ToGeneratedContent for Value {
    fn to_generated_content(&self) -> Result<GeneratedContent, FMError> {
        GeneratedContent::from_value(self)
    }
}

impl FromGeneratedContent for String {
    fn from_generated_content(content: &GeneratedContent) -> Result<Self, FMError> {
        content.value()
    }
}

impl ToGeneratedContent for String {
    fn to_generated_content(&self) -> Result<GeneratedContent, FMError> {
        Ok(GeneratedContent::from(self.clone()))
    }
}

impl ToGeneratedContent for str {
    fn to_generated_content(&self) -> Result<GeneratedContent, FMError> {
        Ok(GeneratedContent::from(self))
    }
}

impl FromGeneratedContent for bool {
    fn from_generated_content(content: &GeneratedContent) -> Result<Self, FMError> {
        content.value()
    }
}

impl ToGeneratedContent for bool {
    fn to_generated_content(&self) -> Result<GeneratedContent, FMError> {
        Ok(GeneratedContent::from(*self))
    }
}

macro_rules! impl_numeric_conversion {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl FromGeneratedContent for $ty {
                fn from_generated_content(content: &GeneratedContent) -> Result<Self, FMError> {
                    content.value()
                }
            }

            impl ToGeneratedContent for $ty {
                fn to_generated_content(&self) -> Result<GeneratedContent, FMError> {
                    Ok(GeneratedContent::from(*self))
                }
            }
        )+
    };
}

impl_numeric_conversion!(f32, f64, i8, i16, i32, i64, u8, u16, u32, u64);

impl<T> FromGeneratedContent for Vec<T>
where
    T: FromGeneratedContent,
{
    fn from_generated_content(content: &GeneratedContent) -> Result<Self, FMError> {
        let values: Vec<Value> = content.value()?;
        values
            .iter()
            .map(|value| {
                let nested = GeneratedContent::try_from(value.clone())?;
                T::from_generated_content(&nested)
            })
            .collect()
    }
}

impl<T> ToGeneratedContent for Vec<T>
where
    T: ToGeneratedContent,
{
    fn to_generated_content(&self) -> Result<GeneratedContent, FMError> {
        let values = self
            .iter()
            .map(ToGeneratedContent::to_generated_content)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(GeneratedContent {
            value: Value::Array(
                values
                    .into_iter()
                    .map(GeneratedContent::into_raw_value)
                    .collect(),
            ),
            generation_id: None,
            is_complete: true,
        })
    }
}

impl<T> FromGeneratedContent for Option<T>
where
    T: FromGeneratedContent,
{
    fn from_generated_content(content: &GeneratedContent) -> Result<Self, FMError> {
        if content.raw_value().is_null() {
            return Ok(None);
        }
        T::from_generated_content(content).map(Some)
    }
}

impl<T> ToGeneratedContent for Option<T>
where
    T: ToGeneratedContent,
{
    fn to_generated_content(&self) -> Result<GeneratedContent, FMError> {
        match self {
            Some(value) => value.to_generated_content(),
            None => GeneratedContent::from_value(Option::<Value>::None),
        }
    }
}
