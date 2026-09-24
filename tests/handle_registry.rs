#![cfg(feature = "macos_26_0")]

use std::collections::HashSet;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use foundation_models::{
    DynamicGenerationProperty, DynamicGenerationSchema, GeneratedContent, GenerationId,
    GenerationOptions, GenerationSchema, LanguageModelSession, Prompt, StructuredStreamEvent,
    SystemLanguageModel, Tool, ToolOutput,
};
use serde_json::json;

extern "C" {
    fn fm_test_bridge_handle_counts(generation_ids: *mut usize, refusals: *mut usize);
}

static REGISTRY: Mutex<()> = Mutex::new(());

fn exclusive() -> MutexGuard<'static, ()> {
    REGISTRY.lock().unwrap_or_else(PoisonError::into_inner)
}

fn counts() -> (usize, usize) {
    let mut generation_ids = 0;
    let mut refusals = 0;
    unsafe { fm_test_bridge_handle_counts(&raw mut generation_ids, &raw mut refusals) };
    (generation_ids, refusals)
}

fn settles_at(baseline: (usize, usize)) -> bool {
    let deadline = Instant::now() + Duration::from_secs(10);
    while counts() != baseline {
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    true
}

fn model_available() -> bool {
    if SystemLanguageModel::is_available() {
        return true;
    }
    eprintln!("SKIP: model unavailable");
    false
}

fn reply_schema() -> GenerationSchema {
    GenerationSchema::from_dynamic(
        DynamicGenerationSchema::object("Reply").with_property(
            "text",
            DynamicGenerationProperty::new(DynamicGenerationSchema::string()),
        ),
        [],
    )
    .expect("schema")
}

fn identified(value: serde_json::Value) -> GeneratedContent {
    GeneratedContent::from_value(value)
        .expect("generated content")
        .with_generation_id(GenerationId::new().expect("generation ID"))
}

#[test]
fn generation_ids_are_released_with_their_last_handle() {
    let _registry = exclusive();
    let baseline = counts();
    let generation_ids = (0..16)
        .map(|_| GenerationId::new().expect("generation ID"))
        .collect::<Vec<_>>();
    assert_eq!(counts(), (baseline.0 + 16, baseline.1));
    let contents = generation_ids
        .iter()
        .cloned()
        .map(|generation_id| GeneratedContent::from("x").with_generation_id(generation_id))
        .collect::<Vec<_>>();
    drop(generation_ids);
    assert_eq!(counts(), (baseline.0 + 16, baseline.1));
    drop(contents);
    assert_eq!(counts(), baseline);
}

#[test]
fn structured_responses_and_streams_return_the_registry_to_its_baseline() {
    if !model_available() {
        return;
    }
    let _registry = exclusive();
    let baseline = counts();
    let session = LanguageModelSession::new().expect("session");
    let schema = reply_schema();
    for _ in 0..3 {
        let response = session
            .respond_generated_with(
                "Reply with a one-word greeting.",
                &schema,
                true,
                GenerationOptions::new(),
            )
            .expect("structured response");
        let held = [
            response.content.generation_id(),
            response.raw_content.generation_id(),
        ]
        .into_iter()
        .flatten()
        .collect::<HashSet<_>>()
        .len();
        assert_eq!(counts(), (baseline.0 + held, baseline.1));
    }
    assert_eq!(counts(), baseline);

    let snapshots = Arc::new(Mutex::new(0_usize));
    let counter = Arc::clone(&snapshots);
    session
        .stream_generated(
            "Reply with a short greeting.",
            &schema,
            true,
            GenerationOptions::new(),
            move |event| {
                if let StructuredStreamEvent::Snapshot(_) = event {
                    *counter.lock().unwrap() += 1;
                }
            },
        )
        .expect("structured stream");
    assert!(*snapshots.lock().unwrap() > 0);
    assert_eq!(counts(), baseline);
    drop(session);
    assert!(settles_at(baseline), "{baseline:?} -> {:?}", counts());
}

#[test]
fn prompts_keep_their_generation_ids_alive_until_swift_has_read_them() {
    if !model_available() {
        return;
    }
    let _registry = exclusive();
    let baseline = counts();
    let session = LanguageModelSession::new().expect("session");
    let mut prompt = Prompt::text("Name the bird in this record in one word: ");
    prompt.push_structured("Record", identified(json!({ "bird": "heron" })));
    let reply = session.respond_prompt(prompt);
    assert!(reply.is_ok(), "{reply:?}");
    drop(session);
    assert!(settles_at(baseline), "{baseline:?} -> {:?}", counts());
}

#[test]
fn tool_outputs_with_generation_ids_return_the_registry_to_its_baseline() {
    if !model_available() {
        return;
    }
    let _registry = exclusive();
    let baseline = counts();
    let calls = Arc::new(Mutex::new(0_usize));
    let counter = Arc::clone(&calls);
    let tool = Tool::new(
        "lookup_code",
        "Return the secret code word.",
        GenerationSchema::generated_content(),
        move |_| {
            *counter.lock().unwrap() += 1;
            Ok(ToolOutput::structured(identified(
                json!({ "code": "heron" }),
            )))
        },
    );
    let session = LanguageModelSession::builder()
        .instructions("Always call the lookup_code tool before answering.")
        .expect("instructions")
        .tool(tool)
        .build()
        .expect("session");
    if let Err(error) = session.respond("What is the secret code word? Use the lookup_code tool.") {
        eprintln!("tool calling failed here: {error}");
    }
    drop(session);
    assert!(settles_at(baseline), "{baseline:?} -> {:?}", counts());
    eprintln!("lookup_code calls: {}", calls.lock().unwrap());
}

#[cfg(feature = "async")]
mod async_registry {
    use super::*;
    use foundation_models::async_api::AsyncSession;

    #[test]
    fn async_requests_return_the_registry_to_its_baseline() {
        if !model_available() {
            return;
        }
        let _registry = exclusive();
        let baseline = counts();
        let session = LanguageModelSession::new().expect("session");
        let schema = reply_schema();
        let response = pollster::block_on(
            AsyncSession::new(&session)
                .respond_generating(
                    "Reply with a one-word greeting.",
                    &schema,
                    true,
                    GenerationOptions::new(),
                )
                .expect("request"),
        );
        assert!(response.is_ok(), "{response:?}");
        drop(response);

        let mut prompt = Prompt::text("Name the bird in this record in one word: ");
        prompt.push_structured("Record", identified(json!({ "bird": "heron" })));
        let reply = pollster::block_on(
            AsyncSession::new(&session)
                .respond(prompt)
                .expect("request"),
        );
        assert!(reply.is_ok(), "{reply:?}");
        drop(reply);

        let pending = AsyncSession::new(&session)
            .respond_generating(
                "Write a very long poem about the sea.",
                &schema,
                true,
                GenerationOptions::new(),
            )
            .expect("request");
        drop(pending);
        let reply = session.respond("Say the word yes.");
        assert!(reply.is_ok(), "{reply:?}");
        drop(session);
        assert!(settles_at(baseline), "{baseline:?} -> {:?}", counts());
    }
}
