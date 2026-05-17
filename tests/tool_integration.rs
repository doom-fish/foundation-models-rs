use foundation_models::{
    FMError, GeneratedContent, GenerationSchema, Prompt, Tool, ToolDefinition, ToolOutput,
};
use serde_json::json;

#[test]
fn tool_metadata_and_outputs_stay_in_sync() -> Result<(), FMError> {
    let parameters = GenerationSchema::generated_content();
    let tool = Tool::new(
        "lookup_weather",
        "Look up the current weather.",
        parameters.clone(),
        |_arguments| Ok(ToolOutput::text("sunny")),
    )
    .with_schema_in_instructions(false);

    assert_eq!(tool.spec().name, "lookup_weather");
    assert_eq!(tool.spec().description, "Look up the current weather.");
    assert_eq!(tool.spec().parameters, parameters);
    assert!(!tool.spec().includes_schema_in_instructions);

    let definition = ToolDefinition::new(
        "lookup_weather",
        "Look up the current weather.",
        GenerationSchema::generated_content(),
    );
    assert_eq!(tool.definition(), definition);
    assert_eq!(tool.spec().definition(), definition);

    let text_output = ToolOutput::text("sunny");
    assert_eq!(text_output.prompt(), &Prompt::text("sunny"));

    let structured = GeneratedContent::from_value(json!({ "status": "ok" }))?;
    let structured_output = ToolOutput::structured(structured.clone());
    assert_eq!(structured_output.prompt(), &Prompt::structured(structured));
    Ok(())
}
