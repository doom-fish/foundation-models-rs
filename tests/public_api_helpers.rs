#![cfg(feature = "macos_26_0")]

use foundation_models::prelude::*;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Choice(String);

impl FromGeneratedContent for Choice {
    fn from_generated_content(content: &GeneratedContent) -> Result<Self, FMError> {
        Ok(Self(content.value()?))
    }
}

impl ToGeneratedContent for Choice {
    fn to_generated_content(&self) -> Result<GeneratedContent, FMError> {
        Ok(GeneratedContent::from(self.0.clone()))
    }
}

impl Generable for Choice {
    fn generation_schema() -> Result<GenerationSchema, FMError> {
        GenerationSchema::from_dynamic(
            DynamicGenerationSchema::any_of_strings("Choice", ["alpha", "beta"]),
            [],
        )
    }
}

#[test]
fn response_format_generating_uses_schema_name() -> Result<(), FMError> {
    let response_format = ResponseFormat::generating::<Choice>()?;
    assert_eq!(response_format.name(), "Choice");

    let actual: Value = serde_json::from_str(response_format.schema().json_schema())
        .map_err(|error| FMError::DecodingFailure(error.to_string()))?;
    let expected: Value = serde_json::from_str(Choice::generation_schema()?.json_schema())
        .map_err(|error| FMError::DecodingFailure(error.to_string()))?;
    assert_eq!(actual, expected);
    Ok(())
}

#[test]
fn tool_generable_infers_schema_and_definition() -> Result<(), FMError> {
    let tool = Tool::generable("echo_choice", "Echo a named choice", |choice: Choice| {
        Ok(choice.0)
    })?;

    assert_eq!(tool.spec().name, "echo_choice");

    let actual_parameters: Value = serde_json::from_str(tool.spec().parameters.json_schema())
        .map_err(|error| FMError::DecodingFailure(error.to_string()))?;
    let expected_parameters: Value =
        serde_json::from_str(Choice::generation_schema()?.json_schema())
            .map_err(|error| FMError::DecodingFailure(error.to_string()))?;
    assert_eq!(actual_parameters, expected_parameters);

    let definition = tool.definition();
    assert_eq!(definition.name, "echo_choice");
    assert_eq!(definition.description, "Echo a named choice");

    let definition_parameters: Value = serde_json::from_str(definition.parameters.json_schema())
        .map_err(|error| FMError::DecodingFailure(error.to_string()))?;
    assert_eq!(definition_parameters, expected_parameters);
    assert_eq!(tool.spec().definition(), definition);
    Ok(())
}

#[cfg(feature = "macos_26_0")]
#[test]
fn any_of_strings_schema_compiles() -> Result<(), FMError> {
    let schema = GenerationSchema::from_dynamic(
        DynamicGenerationSchema::any_of_strings("Priority", ["low", "medium", "high"]),
        [],
    )?;
    let json = schema.json_schema();
    assert!(json.contains("low"));
    assert!(json.contains("high"));
    Ok(())
}

#[cfg(feature = "macos_26_0")]
#[test]
fn array_guides_compile_via_swift_bridge() -> Result<(), FMError> {
    let schema = GenerationSchema::from_dynamic(
        DynamicGenerationSchema::array_of(DynamicGenerationSchema::string()).with_guides([
            GenerationGuide::minimum_count(1),
            GenerationGuide::maximum_count(3),
            GenerationGuide::element(GenerationGuide::string_pattern("^[a-z]+$")),
        ]),
        [],
    )?;
    let json = schema.json_schema();
    assert!(json.contains("pattern"), "compiled schema: {json}");
    assert!(
        json.contains("minItems") || json.contains("minimumItems"),
        "compiled schema: {json}"
    );
    assert!(
        json.contains("maxItems") || json.contains("maximumItems"),
        "compiled schema: {json}"
    );
    Ok(())
}

#[cfg(feature = "macos_26_0")]
#[test]
fn generation_id_attaches_to_generated_content() -> Result<(), FMError> {
    let generation_id = GenerationId::new()?;
    let content = GeneratedContent::from_kind_with_id(
        GeneratedContentKind::String("hello".into()),
        generation_id.clone(),
    )?;

    assert_eq!(content.kind(), GeneratedContentKind::String("hello".into()));
    assert_eq!(content.generation_id_handle(), Some(&generation_id));
    assert_eq!(
        content.generation_id(),
        Some(generation_id.best_effort_string())
    );
    Ok(())
}

#[cfg(feature = "macos_26_0")]
#[test]
fn decimal_round_trips_through_generated_content() -> Result<(), FMError> {
    let decimal = Decimal::new("12.34");
    let content = decimal.to_generated_content()?;
    let decoded = Decimal::from_generated_content(&content)?;

    assert_eq!(decoded, decimal);
    assert!(Decimal::generation_schema()?
        .json_schema()
        .contains("number"));
    Ok(())
}
