use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use crate::boundary::call_across_boundary;

pub unsafe fn c_str_to_string(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    unsafe { CStr::from_ptr(ptr) }
        .to_str()
        .ok()
        .map(str::to_owned)
}

#[no_mangle]
pub extern "C" fn arena_free_string(s: *mut c_char) {
    if s.is_null() {
        return;
    }
    let address = s as usize;
    let _ = call_across_boundary(move || unsafe {
        let _ = CString::from_raw(address as *mut c_char);
    });
}
