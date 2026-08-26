use std::ffi::CStr;
use std::os::raw::c_char;

use arena_ffi::arena_free_string;

pub fn err_text(err: *mut c_char) -> String {
    if err.is_null() {
        return String::new();
    }
    let msg = unsafe { CStr::from_ptr(err).to_string_lossy().into_owned() };
    arena_free_string(err);
    msg
}
