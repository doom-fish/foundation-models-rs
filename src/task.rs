use core::ffi::c_void;
use core::ptr::NonNull;

use crate::ffi;

pub struct SwiftTask(NonNull<c_void>);

unsafe impl Send for SwiftTask {}
unsafe impl Sync for SwiftTask {}

impl SwiftTask {
    pub fn from_raw(ptr: *mut c_void) -> Option<Self> {
        NonNull::new(ptr).map(Self)
    }
}

impl Drop for SwiftTask {
    fn drop(&mut self) {
        unsafe {
            ffi::fm_task_cancel(self.0.as_ptr());
            ffi::fm_object_release(self.0.as_ptr());
        }
    }
}
