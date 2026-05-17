//! JSON-schema and dynamic schema builders for structured generation.

use core::ffi::{c_char, c_void};
use std::collections::BTreeMap;
use std::ffi::CString;
use std::sync::mpsc;

use serde_json::{json, Map, Value};

use crate::content::{FromGeneratedContent, ToGeneratedContent};
use crate::error::FMError;
use crate::ffi;

/// A validated FoundationModels generation schema encoded as JSON Schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationSchema {
    json_schema: String,
}

impl GenerationSchema {
    /// Validate and store a JSON schema definition.
    ///
    /// # Errors
    ///
    /// Returns an [`FMError`] if Apple's `GenerationSchema` rejects the schema.
    pub fn from_json_schema(json_schema: impl Into<String>) -> Result<Self, FMError> {
        let json_schema = json_schema.into();
        let schema_c = CString::new(json_schema.as_str()).map_err(|error| {
            FMError::InvalidArgument(format!(
                "schema JSON contains an interior NUL byte: {error}"
            ))
        })?;
        let mut error_ptr: *mut c_char = core::ptr::null_mut();
        let status =
            unsafe { ffi::fm_generation_schema_validate_json(schema_c.as_ptr(), &mut error_ptr) };
        if status != ffi::status::OK {
            return Err(crate::error::from_swift(status, error_ptr));
        }
        Ok(Self { json_schema })
    }

    /// Create a schema from a dynamic root schema plus optional dependencies.
    ///
    /// # Errors
    ///
    /// Returns an [`FMError`] if the dynamic schema is invalid.
    pub fn from_dynamic(
        root: DynamicGenerationSchema,
        dependencies: impl IntoIterator<Item = DynamicGenerationSchema>,
    ) -> Result<Self, FMError> {
        let request = json!({
            "root": root.to_json_value(),
            "dependencies": dependencies
                .into_iter()
                .map(|schema| schema.to_json_value())
                .collect::<Vec<_>>(),
        });
        let request_json = serde_json::to_string(&request).map_err(|error| {
            FMError::InvalidArgument(format!(
                "dynamic schema request is not JSON-serializable: {error}"
            ))
        })?;
        let request_c = CString::new(request_json).map_err(|error| {
            FMError::InvalidArgument(format!("dynamic schema JSON contains NUL byte: {error}"))
        })?;
        let (tx, rx) = mpsc::channel();
        let tx_box: Box<mpsc::Sender<Result<String, FMError>>> = Box::new(tx);
        let context = Box::into_raw(tx_box).cast::<c_void>();
        unsafe {
            ffi::fm_generation_schema_compile_json(
                request_c.as_ptr(),
                context,
                schema_callback_trampoline,
            );
        }
        let json_schema = rx.recv().map_err(|_| FMError::Unknown {
            code: ffi::status::UNKNOWN,
            message: "Swift bridge dropped the schema callback channel".into(),
        })??;
        Ok(Self { json_schema })
    }

    /// The JSON Schema payload accepted by Apple's `GenerationSchema`.
    #[must_use]
    pub fn json_schema(&self) -> &str {
        &self.json_schema
    }

    /// Best-effort name (the schema's `title`).
    #[must_use]
    pub fn name(&self) -> Option<String> {
        let value: Value = serde_json::from_str(&self.json_schema).ok()?;
        value.get("title")?.as_str().map(ToOwned::to_owned)
    }

    /// A JSON string schema.
    #[must_use]
    pub fn string() -> Self {
        Self::from_json_schema_unchecked(r#"{"type":"string"}"#.into())
    }

    /// A JSON integer schema.
    #[must_use]
    pub fn integer() -> Self {
        Self::from_json_schema_unchecked(r#"{"type":"integer"}"#.into())
    }

    /// A JSON number schema.
    #[must_use]
    pub fn number() -> Self {
        Self::from_json_schema_unchecked(r#"{"type":"number"}"#.into())
    }

    /// A JSON boolean schema.
    #[must_use]
    pub fn boolean() -> Self {
        Self::from_json_schema_unchecked(r#"{"type":"boolean"}"#.into())
    }

    /// A schema for arbitrary JSON (`GeneratedContent`).
    #[must_use]
    pub fn generated_content() -> Self {
        Self::from_json_schema_unchecked(
            r##"{"title":"GeneratedContent","description":"Any legal JSON","anyOf":[{"type":"object","additionalProperties":{"$ref":"#"}},{"type":"array","items":{"$ref":"#"}},{"type":"boolean"},{"type":"number"},{"type":"string"}]}"##.into(),
        )
    }

    pub(crate) fn from_json_schema_unchecked(json_schema: String) -> Self {
        Self { json_schema }
    }
}

/// A dynamic FoundationModels schema description.
#[derive(Debug, Clone, PartialEq)]
pub enum DynamicGenerationSchema {
    Object {
        name: String,
        description: Option<String>,
        properties: BTreeMap<String, DynamicGenerationProperty>,
    },
    Array {
        item: Box<DynamicGenerationSchema>,
        minimum_elements: Option<usize>,
        maximum_elements: Option<usize>,
        guides: Vec<GenerationGuide>,
    },
    AnyOf {
        name: String,
        description: Option<String>,
        choices: Vec<DynamicGenerationSchema>,
    },
    AnyOfStrings {
        name: String,
        description: Option<String>,
        choices: Vec<String>,
    },
    String {
        description: Option<String>,
        guides: Vec<GenerationGuide>,
    },
    Integer {
        description: Option<String>,
        guides: Vec<GenerationGuide>,
    },
    Float {
        description: Option<String>,
        guides: Vec<GenerationGuide>,
    },
    Number {
        description: Option<String>,
        guides: Vec<GenerationGuide>,
    },
    Decimal {
        description: Option<String>,
        guides: Vec<GenerationGuide>,
    },
    Boolean {
        description: Option<String>,
    },
    GeneratedContent {
        description: Option<String>,
    },
    Reference {
        name: String,
    },
}

impl DynamicGenerationSchema {
    /// Create an object schema.
    #[must_use]
    pub fn object(name: impl Into<String>) -> Self {
        Self::Object {
            name: name.into(),
            description: None,
            properties: BTreeMap::new(),
        }
    }

    /// Create a string schema.
    #[must_use]
    pub fn string() -> Self {
        Self::String {
            description: None,
            guides: Vec::new(),
        }
    }

    /// Create an integer schema.
    #[must_use]
    pub fn integer() -> Self {
        Self::Integer {
            description: None,
            guides: Vec::new(),
        }
    }

    /// Create a floating-point schema.
    #[must_use]
    pub fn float() -> Self {
        Self::Float {
            description: None,
            guides: Vec::new(),
        }
    }

    /// Create a number schema.
    #[must_use]
    pub fn number() -> Self {
        Self::Number {
            description: None,
            guides: Vec::new(),
        }
    }

    /// Create a decimal schema.
    #[must_use]
    pub fn decimal() -> Self {
        Self::Decimal {
            description: None,
            guides: Vec::new(),
        }
    }

    /// Create a boolean schema.
    #[must_use]
    pub fn boolean() -> Self {
        Self::Boolean { description: None }
    }

    /// Create an arbitrary-JSON schema.
    #[must_use]
    pub fn generated_content() -> Self {
        Self::GeneratedContent { description: None }
    }

    /// Create an array schema.
    #[must_use]
    pub fn array_of(item: Self) -> Self {
        Self::Array {
            item: Box::new(item),
            minimum_elements: None,
            maximum_elements: None,
            guides: Vec::new(),
        }
    }

    /// Create a reference to a named dependency.
    #[must_use]
    pub fn reference(name: impl Into<String>) -> Self {
        Self::Reference { name: name.into() }
    }

    /// Create a named union of schemas.
    #[must_use]
    pub fn any_of(name: impl Into<String>, choices: Vec<Self>) -> Self {
        Self::AnyOf {
            name: name.into(),
            description: None,
            choices,
        }
    }

    /// Create a named union of constant string choices.
    #[must_use]
    pub fn any_of_strings(
        name: impl Into<String>,
        choices: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self::AnyOfStrings {
            name: name.into(),
            description: None,
            choices: choices.into_iter().map(Into::into).collect(),
        }
    }

    /// Attach a description.
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        match &mut self {
            Self::Object {
                description: slot, ..
            }
            | Self::AnyOf {
                description: slot, ..
            }
            | Self::AnyOfStrings {
                description: slot, ..
            }
            | Self::String {
                description: slot, ..
            }
            | Self::Integer {
                description: slot, ..
            }
            | Self::Float {
                description: slot, ..
            }
            | Self::Number {
                description: slot, ..
            }
            | Self::Decimal {
                description: slot, ..
            }
            | Self::Boolean { description: slot }
            | Self::GeneratedContent { description: slot } => *slot = Some(description.into()),
            Self::Array { .. } | Self::Reference { .. } => {}
        }
        self
    }

    /// Add a property to an object schema.
    #[must_use]
    pub fn with_property(
        mut self,
        name: impl Into<String>,
        property: DynamicGenerationProperty,
    ) -> Self {
        if let Self::Object { properties, .. } = &mut self {
            properties.insert(name.into(), property);
        }
        self
    }

    /// Set the array bounds.
    #[must_use]
    pub fn with_element_bounds(mut self, minimum: Option<usize>, maximum: Option<usize>) -> Self {
        if let Self::Array {
            minimum_elements,
            maximum_elements,
            ..
        } = &mut self
        {
            *minimum_elements = minimum;
            *maximum_elements = maximum;
        }
        self
    }

    /// Attach FoundationModels generation guides.
    #[must_use]
    pub fn with_guides(mut self, guides: impl IntoIterator<Item = GenerationGuide>) -> Self {
        let guides: Vec<_> = guides.into_iter().collect();
        match &mut self {
            Self::String { guides: slot, .. }
            | Self::Integer { guides: slot, .. }
            | Self::Float { guides: slot, .. }
            | Self::Number { guides: slot, .. }
            | Self::Decimal { guides: slot, .. }
            | Self::Array { guides: slot, .. } => *slot = guides,
            Self::Object { .. }
            | Self::AnyOf { .. }
            | Self::AnyOfStrings { .. }
            | Self::Boolean { .. }
            | Self::GeneratedContent { .. }
            | Self::Reference { .. } => {}
        }
        self
    }

    fn to_json_value(&self) -> Value {
        match self {
            Self::Object {
                name,
                description,
                properties,
            } => object_schema_json(name, description, properties),
            Self::Array {
                item,
                minimum_elements,
                maximum_elements,
                guides,
            } => array_schema_json(item, *minimum_elements, *maximum_elements, guides),
            Self::AnyOf {
                name,
                description,
                choices,
            } => named_schema_json(
                "any_of",
                name,
                description,
                Value::Array(choices.iter().map(Self::to_json_value).collect()),
            ),
            Self::AnyOfStrings {
                name,
                description,
                choices,
            } => named_schema_json(
                "any_of",
                name,
                description,
                Value::Array(choices.iter().cloned().map(Value::String).collect()),
            ),
            Self::String {
                description,
                guides,
            } => primitive_schema_json("string", description, guides),
            Self::Integer {
                description,
                guides,
            } => primitive_schema_json("integer", description, guides),
            Self::Float {
                description,
                guides,
            } => primitive_schema_json("float", description, guides),
            Self::Number {
                description,
                guides,
            } => primitive_schema_json("number", description, guides),
            Self::Decimal {
                description,
                guides,
            } => primitive_schema_json("decimal", description, guides),
            Self::Boolean { description } => primitive_schema_json("boolean", description, &[]),
            Self::GeneratedContent { description } => {
                primitive_schema_json("generated_content", description, &[])
            }
            Self::Reference { name } => json!({ "$ref": name }),
        }
    }
}

fn named_schema_json(
    kind: &str,
    name: &str,
    description: &Option<String>,
    choices: Value,
) -> Value {
    let mut map = Map::new();
    map.insert("type".into(), Value::String(kind.into()));
    map.insert("name".into(), Value::String(name.to_string()));
    if let Some(description) = description {
        map.insert("description".into(), Value::String(description.clone()));
    }
    map.insert("choices".into(), choices);
    Value::Object(map)
}

fn object_schema_json(
    name: &str,
    description: &Option<String>,
    properties: &BTreeMap<String, DynamicGenerationProperty>,
) -> Value {
    let property_map = properties
        .iter()
        .map(|(property_name, property)| (property_name.clone(), property.to_json_value()))
        .collect::<Map<String, Value>>();
    let mut map = Map::new();
    map.insert("type".into(), Value::String("object".into()));
    map.insert("name".into(), Value::String(name.to_string()));
    if let Some(description) = description {
        map.insert("description".into(), Value::String(description.clone()));
    }
    map.insert("properties".into(), Value::Object(property_map));
    Value::Object(map)
}

fn array_schema_json(
    item: &DynamicGenerationSchema,
    minimum_elements: Option<usize>,
    maximum_elements: Option<usize>,
    guides: &[GenerationGuide],
) -> Value {
    let mut map = Map::new();
    map.insert("type".into(), Value::String("array".into()));
    map.insert("items".into(), item.to_json_value());
    if let Some(minimum_elements) = minimum_elements {
        map.insert("min".into(), Value::from(minimum_elements));
    }
    if let Some(maximum_elements) = maximum_elements {
        map.insert("max".into(), Value::from(maximum_elements));
    }
    if !guides.is_empty() {
        map.insert(
            "guides".into(),
            Value::Array(guides.iter().map(GenerationGuide::to_json_value).collect()),
        );
    }
    Value::Object(map)
}

/// A property in a dynamic object schema.
#[derive(Debug, Clone, PartialEq)]
pub struct DynamicGenerationProperty {
    pub schema: DynamicGenerationSchema,
    pub description: Option<String>,
    pub optional: bool,
}

impl DynamicGenerationProperty {
    /// Create a property from a nested schema.
    #[must_use]
    pub fn new(schema: DynamicGenerationSchema) -> Self {
        Self {
            schema,
            description: None,
            optional: false,
        }
    }

    /// Mark the property as optional.
    #[must_use]
    pub const fn optional(mut self, optional: bool) -> Self {
        self.optional = optional;
        self
    }

    /// Attach a property description.
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    fn to_json_value(&self) -> Value {
        let mut value = self.schema.to_json_value();
        if let Value::Object(map) = &mut value {
            if let Some(description) = &self.description {
                map.insert("description".into(), Value::String(description.clone()));
            }
            if self.optional {
                map.insert("optional".into(), Value::Bool(true));
            }
        }
        value
    }
}

/// One of Apple's public `GenerationGuide` builders.
#[derive(Debug, Clone, PartialEq)]
pub enum GenerationGuide {
    StringConstant(String),
    StringAnyOf(Vec<String>),
    StringPattern(String),
    MinimumI64(i64),
    MaximumI64(i64),
    RangeI64(i64, i64),
    MinimumF32(f32),
    MaximumF32(f32),
    RangeF32(f32, f32),
    MinimumF64(f64),
    MaximumF64(f64),
    RangeF64(f64, f64),
    MinimumDecimal(String),
    MaximumDecimal(String),
    RangeDecimal(String, String),
    MinimumCount(usize),
    MaximumCount(usize),
    CountRange(usize, usize),
    CountExact(usize),
    Element(Box<GenerationGuide>),
}

impl GenerationGuide {
    #[must_use]
    pub fn string_constant(value: impl Into<String>) -> Self {
        Self::StringConstant(value.into())
    }

    #[must_use]
    pub fn string_any_of(values: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self::StringAnyOf(values.into_iter().map(Into::into).collect())
    }

    #[must_use]
    pub fn string_pattern(pattern: impl Into<String>) -> Self {
        Self::StringPattern(pattern.into())
    }

    #[must_use]
    pub const fn minimum_i64(value: i64) -> Self {
        Self::MinimumI64(value)
    }

    #[must_use]
    pub const fn maximum_i64(value: i64) -> Self {
        Self::MaximumI64(value)
    }

    #[must_use]
    pub const fn range_i64(minimum: i64, maximum: i64) -> Self {
        Self::RangeI64(minimum, maximum)
    }

    #[must_use]
    pub const fn minimum_f32(value: f32) -> Self {
        Self::MinimumF32(value)
    }

    #[must_use]
    pub const fn maximum_f32(value: f32) -> Self {
        Self::MaximumF32(value)
    }

    #[must_use]
    pub const fn range_f32(minimum: f32, maximum: f32) -> Self {
        Self::RangeF32(minimum, maximum)
    }

    #[must_use]
    pub const fn minimum_f64(value: f64) -> Self {
        Self::MinimumF64(value)
    }

    #[must_use]
    pub const fn maximum_f64(value: f64) -> Self {
        Self::MaximumF64(value)
    }

    #[must_use]
    pub const fn range_f64(minimum: f64, maximum: f64) -> Self {
        Self::RangeF64(minimum, maximum)
    }

    #[must_use]
    pub fn minimum_decimal(value: impl Into<String>) -> Self {
        Self::MinimumDecimal(value.into())
    }

    #[must_use]
    pub fn maximum_decimal(value: impl Into<String>) -> Self {
        Self::MaximumDecimal(value.into())
    }

    #[must_use]
    pub fn range_decimal(minimum: impl Into<String>, maximum: impl Into<String>) -> Self {
        Self::RangeDecimal(minimum.into(), maximum.into())
    }

    #[must_use]
    pub const fn minimum_count(count: usize) -> Self {
        Self::MinimumCount(count)
    }

    #[must_use]
    pub const fn maximum_count(count: usize) -> Self {
        Self::MaximumCount(count)
    }

    #[must_use]
    pub const fn count_range(minimum: usize, maximum: usize) -> Self {
        Self::CountRange(minimum, maximum)
    }

    #[must_use]
    pub const fn count(count: usize) -> Self {
        Self::CountExact(count)
    }

    #[must_use]
    pub fn element(guide: GenerationGuide) -> Self {
        Self::Element(Box::new(guide))
    }

    fn to_json_value(&self) -> Value {
        match self {
            Self::StringConstant(value) => json!({ "kind": "constant", "value": value }),
            Self::StringAnyOf(values) => json!({ "kind": "any_of", "values": values }),
            Self::StringPattern(pattern) => json!({ "kind": "pattern", "pattern": pattern }),
            Self::MinimumI64(value) => json!({ "kind": "minimum", "value": value }),
            Self::MaximumI64(value) => json!({ "kind": "maximum", "value": value }),
            Self::RangeI64(minimum, maximum) => {
                json!({ "kind": "range", "min": minimum, "max": maximum })
            }
            Self::MinimumF32(value) => json!({ "kind": "minimum", "value": value }),
            Self::MaximumF32(value) => json!({ "kind": "maximum", "value": value }),
            Self::RangeF32(minimum, maximum) => {
                json!({ "kind": "range", "min": minimum, "max": maximum })
            }
            Self::MinimumF64(value) => json!({ "kind": "minimum", "value": value }),
            Self::MaximumF64(value) => json!({ "kind": "maximum", "value": value }),
            Self::RangeF64(minimum, maximum) => {
                json!({ "kind": "range", "min": minimum, "max": maximum })
            }
            Self::MinimumDecimal(value) => json!({ "kind": "minimum", "value": value }),
            Self::MaximumDecimal(value) => json!({ "kind": "maximum", "value": value }),
            Self::RangeDecimal(minimum, maximum) => {
                json!({ "kind": "range", "min": minimum, "max": maximum })
            }
            Self::MinimumCount(count) => json!({ "kind": "minimum_count", "value": count }),
            Self::MaximumCount(count) => json!({ "kind": "maximum_count", "value": count }),
            Self::CountRange(minimum, maximum) => {
                json!({ "kind": "count", "min": minimum, "max": maximum })
            }
            Self::CountExact(count) => json!({ "kind": "count", "value": count }),
            Self::Element(guide) => json!({ "kind": "element", "guide": guide.to_json_value() }),
        }
    }
}

fn primitive_schema_json(
    kind: &str,
    description: &Option<String>,
    guides: &[GenerationGuide],
) -> Value {
    let mut map = Map::new();
    map.insert("type".into(), Value::String(kind.into()));
    if let Some(description) = description {
        map.insert("description".into(), Value::String(description.clone()));
    }
    if !guides.is_empty() {
        map.insert(
            "guides".into(),
            Value::Array(guides.iter().map(GenerationGuide::to_json_value).collect()),
        );
    }
    Value::Object(map)
}

/// Rust analogue of FoundationModels' `Generable` protocol.
pub trait Generable: Sized + FromGeneratedContent + ToGeneratedContent {
    /// Return the generation schema that describes `Self`.
    fn generation_schema() -> Result<GenerationSchema, FMError>;
}

// SAFETY: `context` is a `Box<mpsc::Sender<...>>` raw pointer created by
// `GenerationSchema::compile`. Swift calls this callback exactly once, so
// there is no double-free risk. `response` and `error` are C strings owned
// by the Swift bridge and only valid for this call.
unsafe extern "C" fn schema_callback_trampoline(
    context: *mut c_void,
    response: *mut c_char,
    error: *mut c_char,
    status: i32,
) {
    let tx = Box::from_raw(context.cast::<mpsc::Sender<Result<String, FMError>>>());
    let result = if status == ffi::status::OK && !response.is_null() {
        let value = core::ffi::CStr::from_ptr(response)
            .to_string_lossy()
            .into_owned();
        ffi::fm_string_free(response);
        Ok(value)
    } else {
        Err(crate::error::from_swift(status, error))
    };
    let _ = tx.send(result);
}
