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
