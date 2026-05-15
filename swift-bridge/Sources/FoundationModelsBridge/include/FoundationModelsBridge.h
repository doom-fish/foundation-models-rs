// Module map / umbrella header for the FoundationModelsBridge target.
// Currently empty — Swift's `@_cdecl` exports plain C symbols that Rust
// declares manually in src/ffi/mod.rs. The header exists so Swift PM can
// build this target as a static library with a public include path.

#ifndef FOUNDATION_MODELS_BRIDGE_H
#define FOUNDATION_MODELS_BRIDGE_H

#endif /* FOUNDATION_MODELS_BRIDGE_H */
