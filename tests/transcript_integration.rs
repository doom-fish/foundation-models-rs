use foundation_models::{
    FMError, GeneratedContent, GenerationOptions, GenerationSchema, Instructions, Prompt,
    ResponseFormat, SamplingMode, Segment, ToolCall, ToolCalls, ToolDefinition, Transcript,
    TranscriptEntry, TranscriptInstructions, TranscriptPrompt, TranscriptResponse,
    TranscriptToolOutput,
};
use serde_json::{json, Value};

#[test]
fn transcript_round_trips_all_entry_kinds() -> Result<(), FMError> {
    let mut instructions_entry = TranscriptInstructions::new(Instructions::from("Answer in JSON"));
    instructions_entry.id = Some("instructions-1".into());
    instructions_entry.tool_definitions.push(ToolDefinition::new(
        "lookup_weather",
        "Look up the current weather.",
        GenerationSchema::generated_content(),
    ));

    let prompt_content = GeneratedContent::from_value(json!({ "topic": "weather" }))?;
    let mut prompt_entry = TranscriptPrompt::new(Prompt::from(vec![
        Segment::text("What is the forecast?"),
        Segment::structure("UserContext", prompt_content),
    ]));
    prompt_entry.id = Some("prompt-1".into());
    prompt_entry.options = GenerationOptions::new()
        .with_temperature(0.2)
        .with_maximum_response_tokens(64)
        .with_sampling(SamplingMode::TopP(0.9))
        .with_sampling_seed(7);
    prompt_entry.response_format = Some(
        ResponseFormat::json_schema(GenerationSchema::generated_content()).with_name("Forecast"),
    );

    let tool_call = ToolCall::new(
        "tool-call-1",
        "lookup_weather",
        GeneratedContent::from_properties([("city", String::from("Stockholm"))])?,
    );
    let tool_calls_entry = ToolCalls {
        id: Some("tool-calls-1".into()),
        calls: vec![tool_call],
    };

    let mut tool_output_entry = TranscriptToolOutput::new(
        "tool-call-1",
        "lookup_weather",
        vec![Segment::structure(
            "ToolResult",
            GeneratedContent::from_value(json!({ "temperatureC": 18, "condition": "sunny" }))?,
        )],
    );
    tool_output_entry.tool_call_id = Some("tool-call-1".into());

    let mut response_entry = TranscriptResponse::new(vec![
        Segment::text("It is sunny and 18°C."),
        Segment::structure(
            "Answer",
            GeneratedContent::from_value(json!({ "summary": "sunny", "temperatureC": 18 }))?,
        ),
    ]);
    response_entry.id = Some("response-1".into());
    response_entry.asset_ids = vec!["asset-1".into()];

    let transcript = Transcript::from_entries(vec![
        TranscriptEntry::Instructions(instructions_entry),
        TranscriptEntry::Prompt(prompt_entry),
        TranscriptEntry::ToolCalls(tool_calls_entry),
        TranscriptEntry::ToolOutput(tool_output_entry),
        TranscriptEntry::Response(response_entry),
    ]);

    let encoded = transcript.to_json_string()?;
    let decoded = Transcript::from_json_str(&encoded)?;
    let encoded_value: Value = serde_json::from_str(&encoded)
        .map_err(|error| FMError::DecodingFailure(error.to_string()))?;
    let reencoded_value: Value = serde_json::from_str(&decoded.to_json_string()?)
        .map_err(|error| FMError::DecodingFailure(error.to_string()))?;

    assert_eq!(reencoded_value, encoded_value);
    assert_eq!(decoded.len(), 5);
    assert_eq!(
        decoded.iter().filter_map(TranscriptEntry::id).collect::<Vec<_>>(),
        vec![
            "instructions-1",
            "prompt-1",
            "tool-calls-1",
            "tool-call-1",
            "response-1",
        ]
    );
    Ok(())
}
