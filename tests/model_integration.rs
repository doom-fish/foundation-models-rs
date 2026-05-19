use foundation_models::{Adapter, FMError, SystemLanguageModel};

#[test]
fn model_helpers_short_circuit_invalid_c_strings() {
    let invalid_name = Adapter::from_name("bad\0adapter");
    assert!(matches!(
        invalid_name,
        Err(FMError::InvalidArgument(ref message))
            if message.contains("adapter name contains NUL byte")
    ));

    assert!(Adapter::compatible_adapter_identifiers("bad\0adapter").is_empty());
    assert!(!SystemLanguageModel::supports_locale("en_US\0"));
}

#[cfg(all(feature = "async", feature = "macos_26_0"))]
#[test]
fn system_model_token_count_is_positive() -> Result<(), FMError> {
    if !SystemLanguageModel::is_available() {
        eprintln!("SKIP: model unavailable");
        return Ok(());
    }

    let count = pollster::block_on(SystemLanguageModel::token_count("hello world"))?;
    assert!(count > 0, "token count should be positive, got {count}");
    Ok(())
}
