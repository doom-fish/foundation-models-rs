use foundation_models::{
    FMError, LanguageModelSession, Prompt, Transcript, TranscriptEntry, TranscriptPrompt,
};

#[test]
fn session_builder_rejects_instructions_and_transcript_together() -> Result<(), FMError> {
    let transcript = Transcript::from_entries(vec![TranscriptEntry::Prompt(
        TranscriptPrompt::new(Prompt::from("hello")),
    )]);

    let result = LanguageModelSession::builder()
        .instructions("Be concise")?
        .transcript(transcript)
        .build();

    assert!(matches!(
        result,
        Err(FMError::InvalidArgument(ref message))
            if message.contains("either instructions or a transcript")
    ));
    Ok(())
}
