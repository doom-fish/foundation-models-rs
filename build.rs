use std::env;
use std::process::Command;

/// Detect the macOS SDK major version via `xcrun --sdk macosx --show-sdk-version`.
///
/// Returns `None` if detection fails.
///
/// **Why `--sdk macosx` is required**: bare `xcrun --show-sdk-version` follows
/// xcrun's notion of the "active developer dir" plus the embedded "default
/// SDK" preference, which can land on `/Library/Developer/CommandLineTools/
/// SDKs/MacOSX.sdk` even when `xcode-select -p` correctly points at a full
/// Xcode install. Forcing `--sdk macosx` resolves the SDK from the active
/// Xcode toolchain instead.
fn detect_sdk_major_version() -> Option<u32> {
    let output = Command::new("xcrun")
        .args(["--sdk", "macosx", "--show-sdk-version"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let version_str = String::from_utf8_lossy(&output.stdout);
    let major = version_str.trim().split('.').next()?;
    major.parse().ok()
}

fn detect_sdk_root() -> Option<String> {
    let output = Command::new("xcrun")
        .args(["--sdk", "macosx", "--show-sdk-path"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Resolve which `-D<MACRO>` flags to pass to the Swift compiler for each
/// enabled `macos_*` Cargo feature.
fn configure_swift_version_defines(sdk_version: Option<u32>) -> Vec<String> {
    // (cargo_feature, min_sdk_major, swift_define)
    let version_features: [(&str, u32, &str); 1] = [(
        "CARGO_FEATURE_MACOS_26_0",
        26,
        "FOUNDATION_MODELS_HAS_MACOS26_SDK",
    )];

    let sdk_at_least = |min: u32| sdk_version.is_some_and(|v| v >= min);

    let mut define_flags: Vec<String> = Vec::new();
    let mut stubbed_features: Vec<&str> = Vec::new();
    for (cargo_feature, min_sdk, swift_define) in version_features {
        if env::var(cargo_feature).is_err() {
            continue;
        }
        if sdk_at_least(min_sdk) {
            define_flags.push(format!("-D{swift_define}"));
        } else {
            stubbed_features.push(cargo_feature.trim_start_matches("CARGO_FEATURE_"));
        }
    }

    if !stubbed_features.is_empty() {
        warn_or_fail_for_stub_mode(sdk_version, &stubbed_features);
    }

    define_flags
}

fn warn_or_fail_for_stub_mode(sdk_version: Option<u32>, stubbed_features: &[&str]) {
    let opt_out = env::var("FOUNDATION_MODELS_ALLOW_STUBBED_BUILD").is_ok();
    let detection_failed = sdk_version.is_none();
    let feature_list = stubbed_features.join(", ").to_lowercase();

    assert!(
        !detection_failed || opt_out,
        "foundation-models: SDK version detection failed but the following \
         version feature(s) were enabled in Cargo.toml: [{feature_list}]. \
         Building would silently produce a binary whose macOS-version-gated APIs \
         are stubbed out and fail at runtime. Resolve this by:\n\
         \n\
           1. Installing the full Xcode and ensuring `xcode-select -p` points at it; or\n\
           2. Setting DEVELOPER_DIR to a valid Xcode path; or\n\
           3. Removing the unused version feature(s) from your Cargo.toml; or\n\
           4. Setting FOUNDATION_MODELS_ALLOW_STUBBED_BUILD=1 to opt into the \
              stubbed-API build (only useful for `cargo doc`/`cargo check` runs).",
    );

    let suffix = if detection_failed {
        " (suppressed via FOUNDATION_MODELS_ALLOW_STUBBED_BUILD)"
    } else {
        ""
    };
    let detected = sdk_version.map_or_else(|| "unknown".to_string(), |v| v.to_string());
    println!(
        "cargo:warning=Cargo feature(s) [{feature_list}] requested but SDK major \
         version ({detected}) is too old{suffix}; the corresponding Swift APIs will \
         be stubbed out.",
    );
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=DOCS_RS");
    println!("cargo:rerun-if-env-changed=DEVELOPER_DIR");
    println!("cargo:rerun-if-env-changed=SDKROOT");
    println!("cargo:rerun-if-env-changed=FOUNDATION_MODELS_ALLOW_STUBBED_BUILD");

    // docs.rs builds on Linux where Swift toolchain and macOS frameworks are
    // unavailable. Skip native compilation – rustdoc only needs type info.
    if env::var("DOCS_RS").is_ok() {
        return;
    }

    println!("cargo:rustc-link-lib=framework=FoundationModels");

    let swift_dir = "swift-bridge";
    let out_dir = env::var("OUT_DIR").unwrap();
    let swift_build_dir = format!("{out_dir}/swift-build");

    println!("cargo:rerun-if-changed={swift_dir}");

    let sdk_version = detect_sdk_major_version();

    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let swift_triple = match target_arch.as_str() {
        "x86_64" => "x86_64-apple-macosx",
        "aarch64" => "arm64-apple-macosx",
        other => panic!(
            "foundation-models: unsupported target arch '{other}'. \
             Expected x86_64 or aarch64."
        ),
    };

    let mut swift_args: Vec<&str> = vec![
        "build",
        "-c",
        "release",
        "--triple",
        swift_triple,
        "--package-path",
        swift_dir,
        "--scratch-path",
        &swift_build_dir,
    ];

    let define_flags = configure_swift_version_defines(sdk_version);
    for flag in &define_flags {
        swift_args.push("-Xswiftc");
        swift_args.push(flag);
    }

    let output = Command::new("swift")
        .args(&swift_args)
        .output()
        .expect("Failed to build Swift bridge");

    if !output.status.success() {
        eprintln!(
            "Swift build STDOUT:\n{}",
            String::from_utf8_lossy(&output.stdout)
        );
        eprintln!(
            "Swift build STDERR:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        panic!(
            "Swift build failed with exit code: {:?}",
            output.status.code()
        );
    }

    link_swift_bridge(&swift_build_dir);
}

fn link_swift_bridge(swift_build_dir: &str) {
    println!("cargo:rustc-link-search=native={swift_build_dir}/release");
    println!("cargo:rustc-link-lib=static=FoundationModelsBridge");

    println!("cargo:rustc-link-lib=framework=Foundation");

    // Add rpath for Swift runtime libraries
    println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");

    // Add rpath + link search paths for the Xcode Swift runtime. Static Swift
    // package archives carry autolink metadata for `swiftCore` /
    // `swift_Concurrency`, but `rustc` does not add the Xcode runtime search
    // paths automatically.
    match Command::new("xcode-select").arg("-p").output() {
        Ok(output) if output.status.success() => {
            let xcode_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let swift_root = format!("{xcode_path}/Toolchains/XcodeDefault.xctoolchain/usr/lib");
            let swift_rpath = format!("{swift_root}/swift/macosx");
            println!("cargo:rustc-link-arg=-Wl,-rpath,{swift_rpath}");
            println!("cargo:rustc-link-search=native={swift_rpath}");
            if let Some(sdk_root) = detect_sdk_root() {
                println!("cargo:rustc-link-search=native={sdk_root}/usr/lib/swift");
            }
            println!("cargo:rustc-link-lib=swiftCore");
            println!("cargo:rustc-link-lib=swift_Concurrency");
        }
        Ok(output) => {
            println!(
                "cargo:warning=`xcode-select -p` exited non-zero (status={:?}); \
                 the Swift Concurrency rpath will not be baked in. Install the \
                 full Xcode (not just Command Line Tools), or set DEVELOPER_DIR.",
                output.status.code()
            );
        }
        Err(err) => {
            println!(
                "cargo:warning=`xcode-select` could not be invoked ({err}); \
                 the Swift Concurrency rpath will not be baked in. Install Xcode \
                 and ensure xcode-select is on PATH."
            );
        }
    }
}
