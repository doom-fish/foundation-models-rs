//! [`GeneratedContent`] — structured model output represented as JSON.

use core::ffi::c_char;
use core::fmt;
use std::collections::BTreeMap;
use std::ffi::{CStr, CString};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Number, Value};

use crate::error::FMError;
use crate::ffi;
use crate::schema::{DynamicGenerationSchema, Generable, GenerationSchema};

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct BridgeGenerationId {
    pub(crate) token: String,
    pub(crate) description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct BridgeGeneratedContent {
    pub(crate) json: String,
    #[serde(rename = "generationID")]
    pub(crate) generation_id: Option<BridgeGenerationId>,
}

fn owned_string(ptr: *mut c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    let value = unsafe { CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned();
    unsafe { ffi::fm_string_free(ptr) };
    value
}

fn call_string_bridge<F>(call: F) -> Result<String, FMError>
where
    F: FnOnce(*mut *mut c_char, *mut *mut c_char) -> i32,
{
    let mut output: *mut c_char = core::ptr::null_mut();
    let mut error: *mut c_char = core::ptr::null_mut();
    let status = call(&mut output, &mut error);
    if status != ffi::status::OK {
        if !output.is_null() {
            unsafe { ffi::fm_string_free(output) };
        }
        return Err(crate::error::from_swift(status, error));
    }
    if output.is_null() {
        return Err(FMError::Unknown {
            code: ffi::status::UNKNOWN,
            message: "Swift bridge returned success without a payload".into(),
        });
    }
    Ok(owned_string(output))
}

/// Rust wrapper for FoundationModels' opaque `GenerationID`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GenerationId {
    token: String,
    description: String,
}

impl GenerationId {
    /// Create a fresh opaque generation identifier.
    ///
    /// # Errors
    ///
    /// Returns an [`FMError`] if the Swift bridge cannot create the identifier.
    pub fn new() -> Result<Self, FMError> {
        let json = call_string_bridge(|output, error| unsafe {
            ffi::fm_generation_id_create(output, error)
        })?;
        let bridge: BridgeGenerationId = serde_json::from_str(&json)
            .map_err(|error| FMError::DecodingFailure(error.to_string()))?;
        Ok(Self::from_bridge(bridge))
    }

    /// Best-effort string representation of the opaque identifier.
    #[must_use]
    pub fn best_effort_string(&self) -> &str {
        &self.description
    }

    pub(crate) fn to_bridge(&self) -> BridgeGenerationId {
        BridgeGenerationId {
            token: self.token.clone(),
            description: self.description.clone(),
        }
    }

    pub(crate) fn from_bridge(bridge: BridgeGenerationId) -> Self {
        Self {
            token: bridge.token,
            description: bridge.description,
        }
    }
}

impl fmt::Display for GenerationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.best_effort_string())
    }
}

/// Rust wrapper for Foundation's `Decimal` generable/content surface.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Decimal {
    value: String,
}

impl Decimal {
    /// Create a decimal wrapper from its canonical string form.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
        }
    }

    /// Borrow the decimal's string form.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.value
    }
}

impl From<String> for Decimal {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for Decimal {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl AsRef<str> for Decimal {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Rust analogue of `GeneratedContent.Kind`.
#[derive(Debug, Clone, PartialEq)]
pub enum GeneratedContentKind {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<GeneratedContent>),
    Structure {
        properties: BTreeMap<String, GeneratedContent>,
        ordered_keys: Vec<String>,
    },
}

impl GeneratedContentKind {
    fn into_value(self) -> Result<Value, FMError> {
        match self {
            Self::Null => Ok(Value::Null),
            Self::Bool(value) => Ok(Value::Bool(value)),
            Self::Number(value) => Number::from_f64(value).map(Value::Number).ok_or_else(|| {
                FMError::InvalidArgument("generated content number must be finite".into())
            }),
            Self::String(value) => Ok(Value::String(value)),
            Self::Array(values) => Ok(Value::Array(
                values
                    .into_iter()
                    .map(GeneratedContent::into_raw_value)
                    .collect(),
            )),
            Self::Structure {
                properties,
                ordered_keys,
            } => {
                let mut object = Map::new();
                for key in ordered_keys {
                    if let Some(value) = properties.get(&key) {
                        object.insert(key, value.raw_value().clone());
                    }
                }
                for (key, value) in properties {
                    if !object.contains_key(&key) {
                        object.insert(key, value.into_raw_value());
                    }
                }
                Ok(Value::Object(object))
            }
        }
    }
}

/// A piece of generated structured content.
///
/// Apple models structured generations as `GeneratedContent`; the Rust wrapper
/// stores the JSON value plus the metadata that Apple's streaming API exposes.
#[derive(Debug, Clone, PartialEq)]
pub struct GeneratedContent {
    value: Value,
    generation_id: Option<GenerationId>,
    is_complete: bool,
}

impl GeneratedContent {
    /// Parse a JSON string into generated content.
    ///
    /// # Errors
    ///
    /// Returns [`FMError::InvalidArgument`] if `json` is not valid JSON.
    pub fn from_json_str(json: &str) -> Result<Self, FMError> {
        Self::from_json_str_with_id(json, None)
    }

    /// Parse a JSON string into generated content with an attached generation ID.
    ///
    /// # Errors
    ///
    /// Returns [`FMError::InvalidArgument`] if `json` is not valid JSON.
    pub fn from_json_str_with_id(
        json: &str,
        generation_id: impl Into<Option<GenerationId>>,
    ) -> Result<Self, FMError> {
        let value = serde_json::from_str(json).map_err(|error| {
            FMError::InvalidArgument(format!("generated content JSON is invalid: {error}"))
        })?;
        Ok(Self {
            value,
            generation_id: generation_id.into(),
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
        Self::from_value_with_id(value, None)
    }

    /// Convert a serializable Rust value into generated content with an ID.
    ///
    /// # Errors
    ///
    /// Returns [`FMError::InvalidArgument`] if `value` cannot be encoded as JSON.
    pub fn from_value_with_id<T>(
        value: T,
        generation_id: impl Into<Option<GenerationId>>,
    ) -> Result<Self, FMError>
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
            generation_id: generation_id.into(),
            is_complete: true,
        })
    }

    /// Build generated content from a `GeneratedContentKind` value.
    ///
    /// # Errors
    ///
    /// Returns [`FMError::InvalidArgument`] if the kind cannot be represented as valid JSON.
    pub fn from_kind(kind: GeneratedContentKind) -> Result<Self, FMError> {
        Self::from_kind_with_id(kind, None)
    }

    /// Build generated content from a `GeneratedContentKind` value with an ID.
    ///
    /// # Errors
    ///
    /// Returns [`FMError::InvalidArgument`] if the kind cannot be represented as valid JSON.
    pub fn from_kind_with_id(
        kind: GeneratedContentKind,
        generation_id: impl Into<Option<GenerationId>>,
    ) -> Result<Self, FMError> {
        Ok(Self {
            value: kind.into_value()?,
            generation_id: generation_id.into(),
            is_complete: true,
        })
    }

    /// Build generated content from a sequence of values.
    ///
    /// # Errors
    ///
    /// Returns [`FMError`] if any element cannot be converted.
    pub fn from_elements<T>(elements: impl IntoIterator<Item = T>) -> Result<Self, FMError>
    where
        T: ToGeneratedContent,
    {
        Self::from_elements_with_id(elements, None)
    }

    /// Build generated content from a sequence of values with an ID.
    ///
    /// # Errors
    ///
    /// Returns [`FMError`] if any element cannot be converted.
    pub fn from_elements_with_id<T>(
        elements: impl IntoIterator<Item = T>,
        generation_id: impl Into<Option<GenerationId>>,
    ) -> Result<Self, FMError>
    where
        T: ToGeneratedContent,
    {
        let values = elements
            .into_iter()
            .map(|value| {
                value
                    .to_generated_content()
                    .map(GeneratedContent::into_raw_value)
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            value: Value::Array(values),
            generation_id: generation_id.into(),
            is_complete: true,
        })
    }

    /// Build generated content from key/value properties.
    ///
    /// Later duplicates overwrite earlier ones.
    ///
    /// # Errors
    ///
    /// Returns [`FMError`] if any value cannot be converted.
    pub fn from_properties<K, V>(
        properties: impl IntoIterator<Item = (K, V)>,
    ) -> Result<Self, FMError>
    where
        K: Into<String>,
        V: ToGeneratedContent,
    {
        Self::from_properties_with_id(properties, None)
    }

    /// Build generated content from key/value properties with an ID.
    ///
    /// Later duplicates overwrite earlier ones.
    ///
    /// # Errors
    ///
    /// Returns [`FMError`] if any value cannot be converted.
    pub fn from_properties_with_id<K, V>(
        properties: impl IntoIterator<Item = (K, V)>,
        generation_id: impl Into<Option<GenerationId>>,
    ) -> Result<Self, FMError>
    where
        K: Into<String>,
        V: ToGeneratedContent,
    {
        let object = properties
            .into_iter()
            .map(|(key, value)| {
                value
                    .to_generated_content()
                    .map(|content| (key.into(), content.into_raw_value()))
            })
            .collect::<Result<Map<String, Value>, _>>()?;
        Ok(Self {
            value: Value::Object(object),
            generation_id: generation_id.into(),
            is_complete: true,
        })
    }

    /// Build generated content from key/value properties, combining duplicates.
    ///
    /// # Errors
    ///
    /// Returns [`FMError`] if any value cannot be converted or `combine` fails.
    pub fn from_properties_with<K, V, F>(
        properties: impl IntoIterator<Item = (K, V)>,
        generation_id: impl Into<Option<GenerationId>>,
        mut combine: F,
    ) -> Result<Self, FMError>
    where
        K: Into<String>,
        V: ToGeneratedContent,
        F: FnMut(GeneratedContent, GeneratedContent) -> Result<GeneratedContent, FMError>,
    {
        let mut object = Map::new();
        for (key, value) in properties {
            let key = key.into();
            let value = value.to_generated_content()?;
            if let Some(existing) = object.remove(&key) {
                let combined = combine(GeneratedContent::try_from(existing)?, value)?;
                object.insert(key, combined.into_raw_value());
            } else {
                object.insert(key, value.into_raw_value());
            }
        }
        Ok(Self {
            value: Value::Object(object),
            generation_id: generation_id.into(),
            is_complete: true,
        })
    }

    /// Build a value from bridge metadata.
    pub(crate) fn from_bridge_payload(
        payload: BridgeGeneratedContent,
        is_complete: bool,
    ) -> Result<Self, FMError> {
        let mut content = Self::from_json_str_with_id(
            &payload.json,
            payload.generation_id.map(GenerationId::from_bridge),
        )?;
        content.is_complete = is_complete;
        Ok(content)
    }

    pub(crate) fn to_bridge_value(&self) -> Result<Value, FMError> {
        serde_json::to_value(BridgeGeneratedContent {
            json: self.json_string()?,
            generation_id: self.generation_id.as_ref().map(GenerationId::to_bridge),
        })
        .map_err(|error| {
            FMError::InvalidArgument(format!(
                "generated content bridge payload is not JSON-serializable: {error}"
            ))
        })
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

    /// Return the typed content kind.
    #[must_use]
    pub fn kind(&self) -> GeneratedContentKind {
        kind_from_value(&self.value)
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
        self.generation_id
            .as_ref()
            .map(GenerationId::best_effort_string)
    }

    /// Borrow the typed generation identifier handle, if one was attached.
    #[must_use]
    pub fn generation_id_handle(&self) -> Option<&GenerationId> {
        self.generation_id.as_ref()
    }

    /// Replace the attached generation identifier.
    #[must_use]
    pub fn with_generation_id(mut self, generation_id: impl Into<Option<GenerationId>>) -> Self {
        self.generation_id = generation_id.into();
        self
    }
}

fn kind_from_value(value: &Value) -> GeneratedContentKind {
    match value {
        Value::Null => GeneratedContentKind::Null,
        Value::Bool(value) => GeneratedContentKind::Bool(*value),
        Value::Number(value) => GeneratedContentKind::Number(value.as_f64().unwrap_or_default()),
        Value::String(value) => GeneratedContentKind::String(value.clone()),
        Value::Array(values) => GeneratedContentKind::Array(
            values
                .iter()
                .cloned()
                .map(|value| {
                    GeneratedContent::try_from(value)
                        .expect("JSON arrays are valid generated content")
                })
                .collect(),
        ),
        Value::Object(map) => GeneratedContentKind::Structure {
            properties: map
                .iter()
                .map(|(key, value)| {
                    (
                        key.clone(),
                        GeneratedContent::try_from(value.clone())
                            .expect("JSON objects are valid generated content"),
                    )
                })
                .collect(),
            ordered_keys: map.keys().cloned().collect(),
        },
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

impl FromGeneratedContent for Decimal {
    fn from_generated_content(content: &GeneratedContent) -> Result<Self, FMError> {
        let json = CString::new(content.json_string()?).map_err(|error| {
            FMError::InvalidArgument(format!(
                "generated content JSON contains an interior NUL byte: {error}"
            ))
        })?;
        let value = call_string_bridge(|output, error| unsafe {
            ffi::fm_decimal_from_generated_content_json(json.as_ptr(), output, error)
        })?;
        Ok(Self::new(value))
    }
}

impl ToGeneratedContent for Decimal {
    fn to_generated_content(&self) -> Result<GeneratedContent, FMError> {
        let decimal = CString::new(self.as_str()).map_err(|error| {
            FMError::InvalidArgument(format!(
                "decimal string contains an interior NUL byte: {error}"
            ))
        })?;
        let json = call_string_bridge(|output, error| unsafe {
            ffi::fm_decimal_to_generated_content_json(decimal.as_ptr(), output, error)
        })?;
        GeneratedContent::from_json_str(&json)
    }
}

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

impl Generable for GeneratedContent {
    fn generation_schema() -> Result<GenerationSchema, FMError> {
        Ok(GenerationSchema::generated_content())
    }
}

impl Generable for String {
    fn generation_schema() -> Result<GenerationSchema, FMError> {
        Ok(GenerationSchema::string())
    }
}

impl Generable for bool {
    fn generation_schema() -> Result<GenerationSchema, FMError> {
        Ok(GenerationSchema::boolean())
    }
}

impl Generable for Decimal {
    fn generation_schema() -> Result<GenerationSchema, FMError> {
        GenerationSchema::from_dynamic(DynamicGenerationSchema::decimal(), [])
    }
}

macro_rules! impl_integer_generable {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl Generable for $ty {
                fn generation_schema() -> Result<GenerationSchema, FMError> {
                    Ok(GenerationSchema::integer())
                }
            }
        )+
    };
}

macro_rules! impl_number_generable {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl Generable for $ty {
                fn generation_schema() -> Result<GenerationSchema, FMError> {
                    Ok(GenerationSchema::number())
                }
            }
        )+
    };
}

impl_integer_generable!(i8, i16, i32, i64, u8, u16, u32, u64);
impl_number_generable!(f32, f64);

impl<T> Generable for Vec<T>
where
    T: Generable,
{
    fn generation_schema() -> Result<GenerationSchema, FMError> {
        let item_schema: Value = serde_json::from_str(T::generation_schema()?.json_schema())
            .map_err(|error| {
                FMError::InvalidArgument(format!("element schema is not valid JSON: {error}"))
            })?;
        Ok(GenerationSchema::from_json_schema_unchecked(
            serde_json::json!({ "type": "array", "items": item_schema }).to_string(),
        ))
    }
}

impl<T> Generable for Option<T>
where
    T: Generable,
{
    fn generation_schema() -> Result<GenerationSchema, FMError> {
        T::generation_schema()
    }
}
