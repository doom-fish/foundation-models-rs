use foundation_models::{GenerationOptions, SamplingMode};

#[test]
fn generation_options_builder_tracks_explicit_overrides() {
    let defaults = GenerationOptions::new();
    assert_eq!(defaults.temperature(), None);
    assert_eq!(defaults.maximum_response_tokens(), None);
    assert_eq!(defaults.sampling(), SamplingMode::Default);
    assert_eq!(defaults.sampling_seed(), None);

    let configured = defaults
        .with_temperature(0.35)
        .with_maximum_response_tokens(128)
        .with_sampling(SamplingMode::TopP(0.8))
        .with_sampling_seed(7);

    assert_eq!(defaults, GenerationOptions::new());
    assert_eq!(configured.temperature(), Some(0.35));
    assert_eq!(configured.maximum_response_tokens(), Some(128));
    assert_eq!(configured.sampling(), SamplingMode::TopP(0.8));
    assert_eq!(configured.sampling_seed(), Some(7));
}

#[cfg(feature = "macos_26_0")]
#[test]
fn explicit_nil_schema_generates_null_fields() -> Result<(), foundation_models::FMError> {
    use foundation_models::{
        DynamicGenerationProperty, DynamicGenerationSchema, GenerationSchema, LanguageModelSession,
        SystemLanguageModel,
    };
    use serde_json::Value;

    if !SystemLanguageModel::is_available() {
        eprintln!("SKIP: model unavailable");
        return Ok(());
    }

    let make_properties = || {
        [
            (
                "title",
                DynamicGenerationProperty::new(DynamicGenerationSchema::string()),
            ),
            (
                "subtitle",
                DynamicGenerationProperty::new(DynamicGenerationSchema::string()).optional(true),
            ),
        ]
    };

    let dynamic_schema = GenerationSchema::from_dynamic(
        DynamicGenerationSchema::new_with_nil_repr(
            "NullableReply",
            Some("Return a title and an explicitly null subtitle.".into()),
            true,
            make_properties(),
        ),
        [],
    )?;
    let typed_schema = GenerationSchema::new_with_nil_repr(
        Some("Return a title and an explicitly null subtitle.".into()),
        true,
        make_properties(),
    )?;

    let typed_json: Value = serde_json::from_str(typed_schema.json_schema())
        .map_err(|error| foundation_models::FMError::DecodingFailure(error.to_string()))?;
    let dynamic_json: Value = serde_json::from_str(dynamic_schema.json_schema())
        .map_err(|error| foundation_models::FMError::DecodingFailure(error.to_string()))?;

    for json in [&typed_json, &dynamic_json] {
        let required = json["required"].as_array().ok_or_else(|| {
            foundation_models::FMError::DecodingFailure("schema is missing a required array".into())
        })?;
        assert!(required.iter().any(|value| value.as_str() == Some("title")));
        assert!(required
            .iter()
            .any(|value| value.as_str() == Some("subtitle")));
        let has_null = json["properties"]["subtitle"]["anyOf"]
            .as_array()
            .is_some_and(|choices| {
                choices
                    .iter()
                    .any(|choice| choice.get("type").and_then(Value::as_str) == Some("null"))
            });
        assert!(has_null, "schema should allow explicit nulls: {json}");
    }

    let session = LanguageModelSession::new();
    let content = session
        .respond_generated_with(
            "Return JSON with title set to Alpha and subtitle set to null. Do not omit subtitle.",
            &dynamic_schema,
            false,
            GenerationOptions::new()
                .with_sampling(SamplingMode::Greedy)
                .with_maximum_response_tokens(64),
        )?
        .content;

    let rendered = content.json_string()?;
    assert!(
        rendered.contains("\"subtitle\":null"),
        "content was {rendered}"
    );
    let subtitle: Option<String> = content.value_for_property("subtitle")?;
    assert_eq!(subtitle, None);
    let title: String = content.value_for_property("title")?;
    assert!(!title.trim().is_empty(), "title should not be empty");
    Ok(())
}
