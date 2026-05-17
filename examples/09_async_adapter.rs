//! Async adapter example — uses `async_api::AsyncAdapter`.
//!
//! Demonstrates `SystemLanguageModel.Adapter init(name:)` and
//! `Adapter.compatibility(for:)` as executor-agnostic Futures.
//!
//! Run with:
//! ```text
//! cargo run --example 09_async_adapter --features "macos_26_0 async"
//! ```

use foundation_models::async_api::AsyncAdapter;
use foundation_models::SystemLanguageModel;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if !SystemLanguageModel::is_available() {
        eprintln!("SKIP: FoundationModels unavailable on this system");
        return Ok(());
    }

    pollster::block_on(async {
        // --- AsyncAdapter::compatibility ---
        // Use a placeholder name; on a headless system the result is typically empty.
        let ids = AsyncAdapter::compatibility("com.example.nonexistent")?.await;
        match ids {
            Ok(list) => println!("compatibility: {list:?}"),
            Err(e) => println!("compatibility (expected on headless): {e}"),
        }

        // --- AsyncAdapter::from_name ---
        // A nonexistent adapter name is expected to fail; handle it gracefully.
        let adapter_result = AsyncAdapter::from_name("com.example.nonexistent")?.await;
        match adapter_result {
            Ok(_adapter) => println!("from_name: adapter loaded"),
            Err(e) => println!("from_name (expected on headless): {e}"),
        }

        Ok::<(), Box<dyn std::error::Error>>(())
    })
}
