use core::fmt;
use core::hash::{Hash, Hasher};

pub struct SwiftHandle {
    token: u64,
    release: unsafe extern "C" fn(u64),
}

impl SwiftHandle {
    pub fn owned(token: u64, release: unsafe extern "C" fn(u64)) -> Self {
        Self { token, release }
    }

    pub fn retained(
        token: u64,
        retain: unsafe extern "C" fn(u64) -> bool,
        release: unsafe extern "C" fn(u64),
    ) -> Option<Self> {
        unsafe { retain(token) }.then(|| Self::owned(token, release))
    }

    pub const fn token(&self) -> u64 {
        self.token
    }
}

impl Drop for SwiftHandle {
    fn drop(&mut self) {
        unsafe { (self.release)(self.token) };
    }
}

impl PartialEq for SwiftHandle {
    fn eq(&self, other: &Self) -> bool {
        self.token == other.token
    }
}

impl Eq for SwiftHandle {}

impl Hash for SwiftHandle {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.token.hash(state);
    }
}

impl fmt::Debug for SwiftHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("SwiftHandle").field(&self.token).finish()
    }
}

#[cfg(all(test, feature = "macos_26_0"))]
mod tests {
    use core::ffi::{c_char, c_void};
    use core::ptr;
    use std::ffi::{CStr, CString};
    use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

    use serde_json::json;

    use crate::content::{BridgeGenerationId, GeneratedContent, GenerationId};
    use crate::error::{from_swift_message, FMError, Refusal};
    use crate::ffi;
    use crate::schema::GenerationSchema;
    use crate::session::{wait_for_bridge, BridgePayload, SessionResponse};
    use crate::tool::{tool_callback_trampoline, Tool, ToolOutput, ToolRegistry};

    type Deliver = unsafe extern "C" fn(*mut c_void, ffi::FmRespondCallback);

    extern "C" {
        fn fm_test_bridge_handle_counts(generation_ids: *mut usize, refusals: *mut usize);
        fn fm_test_deliver_refusal(context: *mut c_void, callback: ffi::FmRespondCallback);
        fn fm_test_deliver_identified_response(
            context: *mut c_void,
            callback: ffi::FmRespondCallback,
        );
        fn fm_test_accept_tool_output(output_json: *const c_char) -> i32;
    }

    static REGISTRY: Mutex<()> = Mutex::new(());

    fn exclusive() -> MutexGuard<'static, ()> {
        REGISTRY.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn counts() -> (usize, usize) {
        let mut generation_ids = 0;
        let mut refusals = 0;
        unsafe { fm_test_bridge_handle_counts(&raw mut generation_ids, &raw mut refusals) };
        (generation_ids, refusals)
    }

    fn delivered<T: BridgePayload>(deliver: Deliver) -> Result<T, FMError> {
        wait_for_bridge(|context, callback| {
            unsafe { deliver(context, callback) };
            ptr::null_mut()
        })
    }

    unsafe extern "C" fn discard(
        _context: *mut c_void,
        response: *mut c_char,
        error: *mut c_char,
        _status: i32,
    ) {
        unsafe {
            ffi::fm_string_free(response);
            ffi::fm_string_free(error);
        }
    }

    #[test]
    fn handle_owners_stay_send_and_sync() {
        fn check<T: Send + Sync>() {}
        check::<GenerationId>();
        check::<Refusal>();
        check::<FMError>();
        check::<SessionResponse<GeneratedContent>>();
    }

    #[test]
    fn generation_ids_leave_the_registry_with_their_last_handle() {
        let _registry = exclusive();
        let (generation_ids, refusals) = counts();
        let generation_id = GenerationId::new().expect("generation ID");
        let content = GeneratedContent::from_json_str_with_id("{}", generation_id.clone())
            .expect("generated content");
        let copy = content.clone();
        assert_eq!(counts(), (generation_ids + 1, refusals));
        drop(generation_id);
        drop(content);
        assert_eq!(counts(), (generation_ids + 1, refusals));
        drop(copy);
        assert_eq!(counts(), (generation_ids, refusals));
    }

    #[test]
    fn responses_own_the_generation_ids_they_carry() {
        let _registry = exclusive();
        let (generation_ids, refusals) = counts();
        let response: SessionResponse<String> =
            delivered(fm_test_deliver_identified_response).expect("response");
        assert_eq!(response.content, "identified");
        let generation_id = response
            .raw_content
            .generation_id()
            .cloned()
            .expect("the raw content carries a generation ID");
        assert_eq!(counts(), (generation_ids + 1, refusals));
        drop(response);
        assert_eq!(counts(), (generation_ids + 1, refusals));
        drop(generation_id);
        assert_eq!(counts(), (generation_ids, refusals));
    }

    #[test]
    fn refusals_leave_the_registry_with_their_last_error() {
        let _registry = exclusive();
        let (generation_ids, refusals) = counts();
        let error = delivered::<String>(fm_test_deliver_refusal).expect_err("a refusal");
        assert!(matches!(error, FMError::Refusal(_)), "{error:?}");
        let refusal = error.refusal().expect("the error carries its refusal");
        assert_eq!(refusal.transcript(), None);
        let copy = error.clone();
        assert_eq!(copy.refusal(), Some(refusal.clone()));
        assert_eq!(counts(), (generation_ids, refusals + 1));
        drop(error);
        drop(copy);
        assert_eq!(counts(), (generation_ids, refusals + 1));
        drop(refusal);
        assert_eq!(counts(), (generation_ids, refusals));
    }

    #[test]
    fn payloads_nobody_decodes_are_reclaimed_after_the_callback() {
        let _registry = exclusive();
        let baseline = counts();
        for _ in 0..64 {
            unsafe {
                fm_test_deliver_identified_response(ptr::null_mut(), discard);
                fm_test_deliver_refusal(ptr::null_mut(), discard);
            }
        }
        assert_eq!(counts(), baseline);
    }

    #[test]
    fn unknown_tokens_are_not_adopted() {
        let _registry = exclusive();
        let baseline = counts();
        let error = from_swift_message(
            ffi::status::REFUSAL,
            json!({ "message": "refused", "refusal": { "token": u64::MAX } }).to_string(),
        );
        assert!(matches!(error, FMError::Refusal(_)), "{error:?}");
        assert_eq!(error.refusal(), None);
        let adopted = GenerationId::adopt(BridgeGenerationId {
            token: u64::MAX,
            description: "stale".into(),
        });
        assert!(
            matches!(adopted, Err(FMError::DecodingFailure(_))),
            "{adopted:?}"
        );
        assert_eq!(counts(), baseline);
    }

    #[test]
    fn repeated_requests_do_not_grow_the_registry() {
        let _registry = exclusive();
        let baseline = counts();
        for _ in 0..64 {
            let response: SessionResponse<String> =
                delivered(fm_test_deliver_identified_response).expect("response");
            let error = delivered::<String>(fm_test_deliver_refusal).expect_err("a refusal");
            assert!(response.raw_content.generation_id().is_some());
            assert!(error.refusal().is_some());
        }
        assert_eq!(counts(), baseline);
    }

    #[test]
    fn tool_outputs_hand_one_reference_per_generation_id_to_swift() {
        let _registry = exclusive();
        let baseline = counts();
        let generation_id = GenerationId::new().expect("generation ID");
        let token = generation_id.token();
        let content = GeneratedContent::from_json_str_with_id("{\"ok\":true}", generation_id)
            .expect("generated content");
        let registry = Arc::new(ToolRegistry::new(vec![Tool::new(
            "echo",
            "Echo structured content.",
            GenerationSchema::generated_content(),
            move |_| Ok(ToolOutput::structured(content.clone())),
        )]));
        let name = CString::new("echo").expect("tool name");
        let arguments = CString::new("{}").expect("tool arguments");
        let mut output: *mut c_char = ptr::null_mut();
        let mut error: *mut c_char = ptr::null_mut();
        let status = unsafe {
            tool_callback_trampoline(
                Arc::as_ptr(&registry).cast_mut().cast(),
                name.as_ptr(),
                arguments.as_ptr(),
                &raw mut output,
                &raw mut error,
            )
        };
        assert_eq!(status, ffi::status::OK);
        assert!(error.is_null());
        let output_json = unsafe { CStr::from_ptr(output) }.to_owned();
        unsafe { ffi::fm_string_free(output) };
        assert!(
            output_json
                .to_string_lossy()
                .contains(&format!("\"token\":{token}")),
            "{output_json:?}"
        );
        drop(registry);
        assert_eq!(counts(), (baseline.0 + 1, baseline.1));
        assert_eq!(
            unsafe { fm_test_accept_tool_output(output_json.as_ptr()) },
            ffi::status::OK
        );
        assert_eq!(counts(), baseline);
        assert_eq!(
            unsafe { fm_test_accept_tool_output(output_json.as_ptr()) },
            ffi::status::INVALID_ARGUMENT
        );
        assert_eq!(counts(), baseline);
    }

    #[cfg(feature = "async")]
    #[test]
    fn futures_release_the_handles_they_received() {
        use crate::async_api::PendingBridge;

        let _registry = exclusive();
        let baseline = counts();
        let pending = PendingBridge::<SessionResponse<String>>::start(|context, callback| {
            unsafe { fm_test_deliver_identified_response(context, callback) };
            ptr::null_mut()
        });
        assert_eq!(counts(), (baseline.0 + 1, baseline.1));
        drop(pending);
        assert_eq!(counts(), baseline);

        let pending = PendingBridge::<String>::start(|context, callback| {
            unsafe { fm_test_deliver_refusal(context, callback) };
            ptr::null_mut()
        });
        let error = pollster::block_on(pending).expect_err("a refusal");
        assert!(error.refusal().is_some());
        assert_eq!(counts(), (baseline.0, baseline.1 + 1));
        drop(error);
        assert_eq!(counts(), baseline);
    }
}
