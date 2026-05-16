//! Exercise the schema / tool helpers added in the v0.7 surface sweep.
//!
//! Run with: `cargo run --example 07_schema_surface --features macos_26_0`

use foundation_models::prelude::*;

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let enum_schema = GenerationSchema::from_dynamic(
        DynamicGenerationSchema::any_of_strings("Priority", ["low", "medium", "high"]),
        [],
    )?;
    println!("enum schema: {}", enum_schema.json_schema());

    let list_schema = GenerationSchema::from_dynamic(
        DynamicGenerationSchema::array_of(DynamicGenerationSchema::string()).with_guides([
            GenerationGuide::minimum_count(1),
            GenerationGuide::maximum_count(3),
            GenerationGuide::element(GenerationGuide::string_pattern("^[a-z]+$")),
        ]),
        [],
    )?;
    println!("guided array schema: {}", list_schema.json_schema());

    let response_format = ResponseFormat::generating::<Choice>()?;
    println!("response format name: {}", response_format.name());

    let generation_id = GenerationId::new()?;
    let decimal = Decimal::new("12.34");
    let decimal_content = decimal
        .to_generated_content()?
        .with_generation_id(generation_id.clone());
    println!(
        "decimal round-trip: {}",
        Decimal::from_generated_content(&decimal_content)?
    );
    println!(
        "generation id: {}",
        decimal_content
            .generation_id()
            .unwrap_or_else(|| generation_id.best_effort_string())
    );

    let tool = Tool::generable(
        "echo_choice",
        "Echo the provided choice",
        |choice: Choice| Ok(choice.0),
    )?;
    let definition = tool.definition();
    println!(
        "tool definition: {} -> {}",
        definition.name, definition.description
    );

    Ok(())
}
