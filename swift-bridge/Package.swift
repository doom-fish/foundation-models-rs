// swift-tools-version:5.9
import PackageDescription

// Swift compiler defines (FOUNDATION_MODELS_HAS_MACOS26_SDK) are passed via
// -Xswiftc flags from build.rs based on Cargo feature flags (macos_26_0).

let package = Package(
    name: "FoundationModelsBridge",
    platforms: [
        .macOS(.v13)
    ],
    products: [
        .library(
            name: "FoundationModelsBridge",
            type: .static,
            targets: ["FoundationModelsBridge"])
    ],
    targets: [
        .target(
            name: "FoundationModelsBridge",
            path: "Sources/FoundationModelsBridge",
            publicHeadersPath: "include")
    ]
)
