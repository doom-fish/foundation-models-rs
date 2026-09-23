//! [`SystemLanguageModel`] — entry point for querying device capability and
//! building configured model handles.

use core::ffi::{c_char, c_void};
use std::ffi::CString;
use std::path::Path;
use std::ptr;

use serde_json::Value;

#[cfg(feature = "async")]
use crate::async_api::PendingText;
use crate::error::{from_swift, FMError, Unavailability};
use crate::ffi;
#[cfg(feature = "async")]
use crate::prompt::{ToInstructions, ToPrompt};
#[cfg(feature = "async")]
use crate::schema::GenerationSchema;
use crate::session::wait_for_bridge_text;
#[cfg(feature = "async")]
use crate::tool::{tool_specs_json, Tool};
#[cfg(feature = "async")]
use crate::transcript::Transcript;

fn availability_from_code(code: i32) -> Availability {
    match code {
        0 => Availability::Available,
        1 => Availability::Unavailable(Unavailability::DeviceNotEligible),
        2 => Availability::Unavailable(Unavailability::AppleIntelligenceNotEnabled),
        3 => Availability::Unavailable(Unavailability::ModelNotReady),
        -1 => Availability::Unavailable(Unavailability::OsTooOld),
        _ => Availability::Unavailable(Unavailability::Unknown),
    }
}

fn owned_string(ptr: *mut c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    let string = unsafe { core::ffi::CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned();
    unsafe { ffi::fm_string_free(ptr) };
    string
}

fn json_string(ptr: *mut c_char) -> String {
    if ptr.is_null() {
        return String::from("[]");
    }
    owned_string(ptr)
}

fn context_size_of(model: *mut c_void) -> usize {
    usize::try_from(unsafe { ffi::fm_system_model_context_size(model) }).unwrap_or(0)
}

#[cfg(feature = "async")]
fn start_token_count(
    model: *mut c_void,
    kind: i32,
    input_json: String,
) -> Result<PendingText, FMError> {
    let input_json = CString::new(input_json).map_err(|error| {
        FMError::InvalidArgument(format!("token count input contains a NUL byte: {error}").into())
    })?;
    Ok(PendingText::start(|context, callback| unsafe {
        ffi::fm_system_model_token_count_json_async(
            model,
            kind,
            input_json.as_ptr(),
            context,
            callback,
        )
    }))
}

#[cfg(feature = "async")]
async fn finish_token_count(pending: PendingText) -> Result<usize, FMError> {
    pending.await?.parse::<usize>().map_err(|error| {
        FMError::DecodingFailure(
            format!("token count bridge returned invalid integer: {error}").into(),
        )
    })
}

/// The on-device system language model namespace.
#[derive(Debug, Clone, Copy)]
pub struct SystemLanguageModel;

impl SystemLanguageModel {
    /// Convenience: `availability() == Availability::Available`.
    #[must_use]
    pub fn is_available() -> bool {
        unsafe { ffi::fm_system_model_is_available() }
    }

    /// Detailed availability state of the default model.
    #[must_use]
    pub fn availability() -> Availability {
        let code = unsafe { ffi::fm_system_model_availability_code() };
        availability_from_code(code)
    }

    /// Borrow the SDK's shared default model as a configured handle.
    #[must_use]
    pub fn default_model() -> Option<ConfiguredSystemLanguageModel> {
        let ptr = unsafe { ffi::fm_system_model_create_default() };
        (!ptr.is_null()).then_some(ConfiguredSystemLanguageModel { ptr })
    }

    /// Build a configured system model for the supplied use case and guardrails.
    ///
    /// # Errors
    ///
    /// Returns an [`FMError`] if the current OS does not expose FoundationModels.
    pub fn with_use_case(
        use_case: UseCase,
        guardrails: Guardrails,
    ) -> Result<ConfiguredSystemLanguageModel, FMError> {
        let mut error: *mut c_char = ptr::null_mut();
        let ptr = unsafe {
            ffi::fm_system_model_create(use_case.as_ffi(), guardrails.as_ffi(), &mut error)
        };
        if ptr.is_null() {
            return Err(from_swift(ffi::status::MODEL_UNAVAILABLE, error));
        }
        Ok(ConfiguredSystemLanguageModel { ptr })
    }

    /// Build a configured system model backed by an adapter.
    ///
    /// # Errors
    ///
    /// Returns an [`FMError`] if the adapter is invalid or the current OS does
    /// not expose FoundationModels.
    pub fn with_adapter(
        adapter: &Adapter,
        guardrails: Guardrails,
    ) -> Result<ConfiguredSystemLanguageModel, FMError> {
        let mut error: *mut c_char = ptr::null_mut();
        let ptr = unsafe {
            ffi::fm_system_model_create_with_adapter(adapter.ptr, guardrails.as_ffi(), &mut error)
        };
        if ptr.is_null() {
            return Err(from_swift(ffi::status::MODEL_UNAVAILABLE, error));
        }
        Ok(ConfiguredSystemLanguageModel { ptr })
    }

    /// Languages supported by the default system model.
    #[must_use]
    pub fn supported_languages() -> Vec<String> {
        let json = unsafe { ffi::fm_system_model_supported_languages_json(ptr::null_mut()) };
        serde_json::from_str(&json_string(json)).unwrap_or_default()
    }

    /// Whether the default model supports a locale.
    #[must_use]
    pub fn supports_locale(locale_identifier: &str) -> bool {
        CString::new(locale_identifier).map_or(false, |locale| unsafe {
            ffi::fm_system_model_supports_locale(ptr::null_mut(), locale.as_ptr())
        })
    }

    #[must_use]
    pub fn context_size() -> usize {
        context_size_of(ptr::null_mut())
    }

    /// Count how many tokens the default system model would consume for a prompt.
    ///
    /// # Errors
    ///
    /// Returns an [`FMError`] if the prompt is invalid or the SDK rejects the request.
    #[cfg(feature = "async")]
    #[cfg_attr(docsrs, doc(cfg(feature = "async")))]
    pub async fn token_count(prompt: impl ToPrompt) -> Result<usize, FMError> {
        let pending = start_token_count(
            ptr::null_mut(),
            ffi::token_count_input::PROMPT,
            prompt.to_prompt()?.to_bridge_json()?,
        )?;
        finish_token_count(pending).await
    }
}

/// A configured `SystemLanguageModel` instance.
pub struct ConfiguredSystemLanguageModel {
    pub(crate) ptr: *mut c_void,
}

impl ConfiguredSystemLanguageModel {
    /// Detailed availability of this configured model.
    #[must_use]
    pub fn availability(&self) -> Availability {
        availability_from_code(unsafe { ffi::fm_system_model_availability_code_for(self.ptr) })
    }

    /// Convenience: `availability() == Availability::Available`.
    #[must_use]
    pub fn is_available(&self) -> bool {
        matches!(self.availability(), Availability::Available)
    }

    /// Supported languages for this configured model.
    #[must_use]
    pub fn supported_languages(&self) -> Vec<String> {
        let json = unsafe { ffi::fm_system_model_supported_languages_json(self.ptr) };
        serde_json::from_str(&json_string(json)).unwrap_or_default()
    }

    /// Whether this configured model supports a locale.
    #[must_use]
    pub fn supports_locale(&self, locale_identifier: &str) -> bool {
        CString::new(locale_identifier).map_or(false, |locale| unsafe {
            ffi::fm_system_model_supports_locale(self.ptr, locale.as_ptr())
        })
    }

    #[must_use]
    pub fn context_size(&self) -> usize {
        context_size_of(self.ptr)
    }

    /// Count how many tokens this configured model would consume for a prompt.
    ///
    /// # Errors
    ///
    /// Returns an [`FMError`] if the prompt is invalid or the SDK rejects the request.
    #[cfg(feature = "async")]
    #[cfg_attr(docsrs, doc(cfg(feature = "async")))]
    #[allow(clippy::future_not_send)]
    pub async fn token_count(&self, prompt: impl ToPrompt) -> Result<usize, FMError> {
        let pending = start_token_count(
            self.ptr,
            ffi::token_count_input::PROMPT,
            prompt.to_prompt()?.to_bridge_json()?,
        )?;
        finish_token_count(pending).await
    }

    #[cfg(feature = "async")]
    #[cfg_attr(docsrs, doc(cfg(feature = "async")))]
    #[allow(clippy::future_not_send)]
    pub async fn token_count_for_instructions(
        &self,
        instructions: impl ToInstructions,
    ) -> Result<usize, FMError> {
        let pending = start_token_count(
            self.ptr,
            ffi::token_count_input::INSTRUCTIONS,
            instructions.to_instructions()?.to_bridge_json()?,
        )?;
        finish_token_count(pending).await
    }

    #[cfg(feature = "async")]
    #[cfg_attr(docsrs, doc(cfg(feature = "async")))]
    #[allow(clippy::future_not_send)]
    pub async fn token_count_for_tools(&self, tools: &[Tool]) -> Result<usize, FMError> {
        let pending = start_token_count(
            self.ptr,
            ffi::token_count_input::TOOLS,
            tool_specs_json(tools)?,
        )?;
        finish_token_count(pending).await
    }

    #[cfg(feature = "async")]
    #[cfg_attr(docsrs, doc(cfg(feature = "async")))]
    #[allow(clippy::future_not_send)]
    pub async fn token_count_for_schema(
        &self,
        schema: &GenerationSchema,
    ) -> Result<usize, FMError> {
        let pending = start_token_count(
            self.ptr,
            ffi::token_count_input::SCHEMA,
            schema.bridge_request_json().to_owned(),
        )?;
        finish_token_count(pending).await
    }

    #[cfg(feature = "async")]
    #[cfg_attr(docsrs, doc(cfg(feature = "async")))]
    #[allow(clippy::future_not_send)]
    pub async fn token_count_for_transcript(
        &self,
        transcript: &Transcript,
    ) -> Result<usize, FMError> {
        let pending = start_token_count(
            self.ptr,
            ffi::token_count_input::TRANSCRIPT,
            transcript.to_json_string()?,
        )?;
        finish_token_count(pending).await
    }
}

impl Drop for ConfiguredSystemLanguageModel {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { ffi::fm_object_release(self.ptr) };
        }
    }
}

impl core::fmt::Debug for ConfiguredSystemLanguageModel {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ConfiguredSystemLanguageModel")
            .field("availability", &self.availability())
            .finish()
    }
}

/// One of the public system-model use cases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UseCase {
    /// The default general-purpose model.
    General,
    /// Optimized for content-tagging style prompts.
    ContentTagging,
}

impl UseCase {
    const fn as_ffi(self) -> i32 {
        match self {
            Self::General => 0,
            Self::ContentTagging => 1,
        }
    }
}

/// One of the public system-model guardrail configurations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Guardrails {
    /// The SDK default guardrail policy.
    Default,
    /// A looser policy for content transformation tasks.
    PermissiveContentTransformations,
}

impl Guardrails {
    const fn as_ffi(self) -> i32 {
        match self {
            Self::Default => 0,
            Self::PermissiveContentTransformations => 1,
        }
    }
}

/// A system model adapter.
pub struct Adapter {
    pub(crate) ptr: *mut c_void,
}

impl Adapter {
    /// Load an adapter from a file path.
    ///
    /// # Errors
    ///
    /// Returns an [`FMError`] if the adapter file is invalid.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, FMError> {
        let path = CString::new(path.as_ref().to_string_lossy().into_owned()).map_err(|error| {
            FMError::InvalidArgument(
                format!("adapter path contains an interior NUL byte: {error}").into(),
            )
        })?;
        let mut error: *mut c_char = ptr::null_mut();
        let ptr = unsafe { ffi::fm_adapter_create_from_file(path.as_ptr(), &mut error) };
        if ptr.is_null() {
            return Err(from_swift(ffi::status::ADAPTER_INVALID_ASSET, error));
        }
        Ok(Self { ptr })
    }

    /// Load a named adapter.
    ///
    /// # Errors
    ///
    /// Returns an [`FMError`] if the adapter name is invalid.
    pub fn from_name(name: &str) -> Result<Self, FMError> {
        let name = CString::new(name).map_err(|error| {
            FMError::InvalidArgument(format!("adapter name contains NUL byte: {error}").into())
        })?;
        let mut error: *mut c_char = ptr::null_mut();
        let ptr = unsafe { ffi::fm_adapter_create_from_name(name.as_ptr(), &mut error) };
        if ptr.is_null() {
            return Err(from_swift(ffi::status::ADAPTER_INVALID_NAME, error));
        }
        Ok(Self { ptr })
    }

    /// Compile the adapter.
    ///
    /// # Errors
    ///
    /// Returns an [`FMError`] if compilation fails.
    pub fn compile(&self) -> Result<(), FMError> {
        wait_for_bridge_text(|context, callback| unsafe {
            ffi::fm_adapter_compile(self.ptr, context, callback)
        })
        .map(drop)
    }

    /// Creator-defined metadata as raw JSON.
    #[must_use]
    pub fn creator_defined_metadata_json(&self) -> String {
        let ptr = unsafe { ffi::fm_adapter_metadata_json(self.ptr) };
        owned_string(ptr)
    }

    /// Creator-defined metadata as a `serde_json::Value`.
    pub fn creator_defined_metadata(&self) -> Result<Value, FMError> {
        serde_json::from_str(&self.creator_defined_metadata_json())
            .map_err(|error| FMError::DecodingFailure(error.to_string().into()))
    }

    /// Compatible adapter identifiers for a logical adapter name.
    #[must_use]
    pub fn compatible_adapter_identifiers(name: &str) -> Vec<String> {
        let Ok(name) = CString::new(name) else {
            return Vec::new();
        };
        let ptr = unsafe { ffi::fm_adapter_compatible_identifiers_json(name.as_ptr()) };
        serde_json::from_str(&json_string(ptr)).unwrap_or_default()
    }

    /// Remove obsolete compiled adapters.
    ///
    /// # Errors
    ///
    /// Returns an [`FMError`] if cleanup fails.
    pub fn remove_obsolete_adapters() -> Result<(), FMError> {
        let mut error: *mut c_char = ptr::null_mut();
        let status = unsafe { ffi::fm_adapter_remove_obsolete(&mut error) };
        if status != ffi::status::OK {
            return Err(from_swift(status, error));
        }
        Ok(())
    }
}

impl Drop for Adapter {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { ffi::fm_object_release(self.ptr) };
        }
    }
}

impl core::fmt::Debug for Adapter {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Adapter").finish_non_exhaustive()
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
