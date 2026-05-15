//! [`SystemLanguageModel`] — entry point for querying device capability.

use crate::error::Unavailability;
use crate::ffi;

/// The on-device default language model provided by the operating system.
///
/// `FoundationModels` exposes a single shared `SystemLanguageModel.default`;
/// this type mirrors that singleton and only carries availability queries.
/// To actually generate text, construct a [`crate::LanguageModelSession`].
///
/// # Examples
///
/// ```rust,no_run
/// use foundation_models::SystemLanguageModel;
///
/// if SystemLanguageModel::is_available() {
///     println!("Apple Intelligence model is ready.");
/// } else {
///     eprintln!("Unavailable: {:?}", SystemLanguageModel::availability());
/// }
/// ```
#[derive(Debug, Clone, Copy)]
pub struct SystemLanguageModel;

impl SystemLanguageModel {
    /// Convenience: `availability() == Availability::Available`.
    #[must_use]
    pub fn is_available() -> bool {
        unsafe { ffi::fm_system_model_is_available() }
    }

    /// Detailed availability state of the on-device model.
    #[must_use]
    pub fn availability() -> Availability {
        let code = unsafe { ffi::fm_system_model_availability_code() };
        match code {
            0 => Availability::Available,
            1 => Availability::Unavailable(Unavailability::DeviceNotEligible),
            2 => Availability::Unavailable(Unavailability::AppleIntelligenceNotEnabled),
            3 => Availability::Unavailable(Unavailability::ModelNotReady),
            -1 => Availability::Unavailable(Unavailability::OsTooOld),
            _ => Availability::Unavailable(Unavailability::Unknown),
        }
    }
}

/// Result of [`SystemLanguageModel::availability`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Availability {
    /// Model is loaded and ready to generate.
    Available,
    /// Model cannot be used; the inner value explains why.
    Unavailable(Unavailability),
}
