//! Integration tests for `async_api` (Tier 1).
//!
//! Run with:
//! ```text
//! cargo test --all-features --test async_api_tests
//! ```

#[cfg(all(feature = "async", feature = "macos_26_0"))]
mod async_api_tests {
    use foundation_models::async_api::{AsyncAdapter, AsyncSession};
    use foundation_models::schema::{
        DynamicGenerationProperty, DynamicGenerationSchema, GenerationSchema,
    };
    use foundation_models::{GenerationOptions, LanguageModelSession, SystemLanguageModel};

    fn model_available() -> bool {
        SystemLanguageModel::is_available()
    }

    // -----------------------------------------------------------------------
    // AsyncSession::respond — happy path
    // -----------------------------------------------------------------------

    #[test]
    fn respond_returns_non_empty_string() {
        if !model_available() {
            eprintln!("SKIP: model unavailable");
            return;
        }
        let session = LanguageModelSession::new();
        let result = pollster::block_on(async {
            AsyncSession::new(&session)
                .respond("Say exactly the word: hello")?
                .await
        });
        let response = result.expect("respond should succeed");
        assert!(
            !response.content.is_empty(),
            "response content must not be empty"
        );
    }

    // -----------------------------------------------------------------------
    // AsyncSession::respond_with_options — happy path
    // -----------------------------------------------------------------------

    #[test]
    fn respond_with_options_returns_result() {
        if !model_available() {
            eprintln!("SKIP: model unavailable");
            return;
        }
        let session = LanguageModelSession::new();
        let result = pollster::block_on(async {
            AsyncSession::new(&session)
                .respond_with_options("Say yes.", GenerationOptions::new())?
                .await
        });
        assert!(result.is_ok(), "respond_with_options should succeed");
    }

    // -----------------------------------------------------------------------
    // AsyncSession::respond_generating — structured happy path
    // -----------------------------------------------------------------------

    #[test]
    fn respond_generating_returns_generated_content() {
        if !model_available() {
            eprintln!("SKIP: model unavailable");
            return;
        }
        let schema = DynamicGenerationSchema::object("Reply").with_property(
            "text",
            DynamicGenerationProperty::new(DynamicGenerationSchema::string())
                .with_description("short reply"),
        );
        let gs = GenerationSchema::from_dynamic(schema, []).expect("schema build");

        let session = LanguageModelSession::new();
        let result = pollster::block_on(async {
            AsyncSession::new(&session)
                .respond_generating("Reply with one word.", &gs, true, GenerationOptions::new())?
                .await
        });
        assert!(result.is_ok(), "respond_generating should succeed");
    }

    // -----------------------------------------------------------------------
    // AsyncAdapter — NUL-byte validation (no Swift call, always safe to run)
    // -----------------------------------------------------------------------

    /// `from_name` with an embedded NUL byte must return an immediate Rust-side
    /// error without ever calling into Swift.
    #[test]
    fn adapter_from_name_nul_byte_returns_error() {
        let result = AsyncAdapter::from_name("bad\0name");
        assert!(result.is_err(), "NUL byte in name must return an error");
    }

    /// `compatibility` with an embedded NUL byte must return an immediate Rust-side
    /// error without ever calling into Swift.
    #[test]
    fn adapter_compatibility_nul_byte_returns_error() {
        let result = AsyncAdapter::compatibility("bad\0name");
        assert!(result.is_err(), "NUL byte in name must return an error");
    }

    // -----------------------------------------------------------------------
    // AsyncAdapter — live adapter tests
    //
    // These tests call into Swift's Adapter API, which internally uses
    // BackgroundAssets.  BackgroundAssets requires a real app bundle and
    // crashes with "main bundle lacks an ID" in a headless test binary.
    // They are therefore gated behind the `FM_LIVE_ADAPTER_TESTS` env-var
    // (set e.g. by an on-device Xcode test runner) and skipped by default.
    // -----------------------------------------------------------------------

    #[test]
    fn adapter_compatibility_returns_vec() {
        if std::env::var("FM_LIVE_ADAPTER_TESTS").is_err() {
            eprintln!("SKIP: set FM_LIVE_ADAPTER_TESTS=1 to run live adapter tests");
            return;
        }
        if !model_available() {
            eprintln!("SKIP: model unavailable");
            return;
        }
        let result = pollster::block_on(async {
            AsyncAdapter::compatibility("com.example.nonexistent")?.await
        });
        match result {
            Ok(ids) => assert!(
                ids.is_empty() || ids.iter().all(|s| !s.is_empty()),
                "ids must be non-empty strings"
            ),
            Err(e) => println!("expected error for unknown adapter: {e}"),
        }
    }

    #[test]
    fn adapter_from_name_error_path() {
        if std::env::var("FM_LIVE_ADAPTER_TESTS").is_err() {
            eprintln!("SKIP: set FM_LIVE_ADAPTER_TESTS=1 to run live adapter tests");
            return;
        }
        if !model_available() {
            eprintln!("SKIP: model unavailable");
            return;
        }
        let result = pollster::block_on(async {
            AsyncAdapter::from_name("com.example.definitely.not.installed")?.await
        });
        assert!(
            result.is_err(),
            "loading a nonexistent adapter must return an error"
        );
    }
}
