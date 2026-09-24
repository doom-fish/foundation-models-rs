#![cfg(feature = "macos_26_0")]

use std::sync::{Arc, Mutex};

use foundation_models::{
    Adapter, FMError, GenerationSchema, LanguageModelSession, Segment, StreamEvent,
    SystemLanguageModel, Tool, ToolOutput, TranscriptEntry,
};

fn model_available() -> bool {
    if SystemLanguageModel::is_available() {
        return true;
    }
    eprintln!("SKIP: model unavailable");
    false
}

#[test]
fn file_urls_that_do_not_parse_return_an_error() {
    let error =
        Adapter::from_file("file://host:port/adapter.fmadapter").expect_err("malformed file URL");
    assert!(
        matches!(error, FMError::AdapterInvalidAsset(_)),
        "{error:?}"
    );
    assert!(
        error.message().contains("not a valid file URL"),
        "{error:?}"
    );
}

#[test]
fn named_adapters_fail_cleanly_without_an_app_bundle() {
    let error = Adapter::from_name("com.example.definitely.not.installed")
        .expect_err("a headless test binary has no named adapters");
    assert!(!error.message().is_empty(), "{error:?}");
    assert!(
        Adapter::compatible_adapter_identifiers("com.example.definitely.not.installed").is_empty()
    );
}

#[test]
fn the_default_model_reports_a_context_size() {
    if !model_available() {
        return;
    }
    assert!(SystemLanguageModel::context_size() > 0);
    let model = SystemLanguageModel::default_model().expect("default model");
    assert_eq!(model.context_size(), SystemLanguageModel::context_size());
}

#[test]
fn a_panicking_stream_callback_cancels_generation() {
    if !model_available() {
        return;
    }
    let session = LanguageModelSession::new().expect("session");
    let result = session.stream("Write a long story about a lighthouse keeper.", |event| {
        assert!(
            !matches!(event, StreamEvent::Chunk(_)),
            "stop at the first chunk"
        );
    });
    assert!(result.is_err(), "a panicking callback must end the stream");

    let reply = session.respond("Say the word yes.");
    assert!(
        !matches!(reply, Err(FMError::ConcurrentRequests(_))),
        "the cancelled stream must not block the next request: {reply:?}"
    );
    assert!(reply.is_ok(), "{reply:?}");
}

#[test]
fn concatenated_stream_chunks_equal_the_final_reply() {
    if !model_available() {
        return;
    }
    let session = LanguageModelSession::new().expect("session");
    let received = Arc::new(Mutex::new(String::new()));
    let sink = Arc::clone(&received);
    session
        .stream(
            "Reply with a short greeting that includes the emoji \u{1F44B}\u{1F3FD} and the word caf\u{e9}.",
            move |event| match event {
                StreamEvent::Chunk(delta) => sink.lock().unwrap().push_str(delta),
                StreamEvent::Replace(text) => text.clone_into(&mut sink.lock().unwrap()),
                _ => {}
            },
        )
        .expect("stream");

    let transcript = session.transcript().expect("transcript");
    let final_reply = transcript
        .iter()
        .filter_map(|entry| match entry {
            TranscriptEntry::Response(response) => Some(
                response
                    .segments
                    .iter()
                    .filter_map(|segment| match segment {
                        Segment::Text(text) => Some(text.text.as_str()),
                        Segment::Structure(_) => None,
                    })
                    .collect::<String>(),
            ),
            _ => None,
        })
        .last()
        .expect("a response entry");
    assert_eq!(*received.lock().unwrap(), final_reply);
}

#[cfg(feature = "async")]
mod async_lifecycle {
    use super::*;
    use foundation_models::async_api::AsyncSession;
    use foundation_models::{
        DynamicGenerationProperty, DynamicGenerationSchema, Instructions, Transcript,
        TranscriptPrompt,
    };

    #[test]
    fn dropping_a_future_cancels_its_generation() {
        if !model_available() {
            return;
        }
        let session = LanguageModelSession::new().expect("session");
        let pending = AsyncSession::new(&session)
            .respond("Write a very long essay about the history of timekeeping.")
            .expect("request");
        drop(pending);

        let reply = session.respond("Say the word yes.");
        assert!(
            !matches!(reply, Err(FMError::ConcurrentRequests(_))),
            "the dropped future must not block the next request: {reply:?}"
        );
        assert!(reply.is_ok(), "{reply:?}");
    }

    #[test]
    fn a_future_with_tools_outlives_its_session() {
        if !model_available() {
            return;
        }
        let calls = Arc::new(Mutex::new(0_usize));
        let counter = Arc::clone(&calls);
        let tool = Tool::new(
            "lookup_code",
            "Return the secret code word.",
            GenerationSchema::generated_content(),
            move |_| {
                *counter.lock().unwrap() += 1;
                Ok(ToolOutput::text("the code word is heron"))
            },
        );
        let session = LanguageModelSession::builder()
            .instructions("Always call the lookup_code tool before answering.")
            .expect("instructions")
            .tool(tool)
            .build()
            .expect("session");
        let pending = AsyncSession::new(&session)
            .respond("What is the secret code word? Use the lookup_code tool.")
            .expect("request");
        drop(session);

        let reply = pollster::block_on(pending);
        assert!(
            !matches!(reply, Err(FMError::Cancelled)),
            "dropping the session must not cancel the request: {reply:?}"
        );
        if let Ok(reply) = reply {
            assert!(!reply.content.is_empty());
        }
    }

    #[test]
    fn token_counts_cover_every_input_kind() {
        if !model_available() {
            return;
        }
        let model = SystemLanguageModel::default_model().expect("default model");
        let schema = GenerationSchema::from_dynamic(
            DynamicGenerationSchema::object("Reply").with_property(
                "text",
                DynamicGenerationProperty::new(DynamicGenerationSchema::string()),
            ),
            [],
        )
        .expect("schema");
        let tool = Tool::new(
            "lookup_weather",
            "Look up the current weather for a city.",
            schema.clone(),
            |_| Ok(ToolOutput::text("sunny")),
        );
        let transcript = Transcript::from_entries(vec![TranscriptEntry::Prompt(
            TranscriptPrompt::new("What is the weather like?".into()),
        )]);

        let counts = pollster::block_on(async {
            Ok::<_, FMError>([
                SystemLanguageModel::token_count("hello world").await?,
                model.token_count("hello world").await?,
                model
                    .token_count_for_instructions(Instructions::from("Answer briefly."))
                    .await?,
                model.token_count_for_tools(&[tool]).await?,
                model.token_count_for_schema(&schema).await?,
                model.token_count_for_transcript(&transcript).await?,
            ])
        });
        match counts {
            Ok(counts) => assert!(counts.iter().all(|&count| count > 0), "{counts:?}"),
            Err(FMError::ModelUnavailable { message, .. }) if message.contains("26.4") => {
                eprintln!("SKIP: token counting needs macOS 26.4");
            }
            Err(error) => panic!("token count failed: {error:?}"),
        }
    }
}
