use arena_ffi::strings::c_str_to_string;
use arena_ffi::arena_free_string;
use std::ffi::CString;

#[test]
fn c_str_to_string_null_ptr_returns_none() {
    let result = unsafe { c_str_to_string(std::ptr::null()) };
    assert_eq!(result, None);
}

#[test]
fn c_str_to_string_valid_utf8_returns_string() {
    let s = CString::new("hello").unwrap();
    let result = unsafe { c_str_to_string(s.as_ptr()) };
    assert_eq!(result, Some("hello".to_string()));
}

#[test]
fn c_str_to_string_invalid_utf8_returns_none() {
    let bytes = vec![0xFF, 0xFE, 0x00];
    let s = unsafe { CString::from_vec_with_nul_unchecked(bytes) };
    let result = unsafe { c_str_to_string(s.as_ptr()) };
    assert_eq!(result, None);
}

#[test]
fn arena_free_string_null_does_not_panic() {
    arena_free_string(std::ptr::null_mut());
}

#[test]
fn arena_free_string_valid_ptr_frees_without_panic() {
    let s = CString::new("owned").unwrap();
    arena_free_string(s.into_raw());
}
