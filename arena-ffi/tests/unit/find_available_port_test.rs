use std::ffi::CStr;
use std::net::TcpListener;
use std::os::raw::c_char;

use arena_ffi::{arena_find_available_port, arena_free_string, ArenaStatus};

#[test]
fn arena_find_available_port_random_returns_port_via_out_param() {
    let mut port_out: i32 = 0;
    let mut err: *mut c_char = std::ptr::null_mut();

    let status = arena_find_available_port(22000, 22100, 0, &mut port_out, &mut err);

    assert_eq!(status, ArenaStatus::Ok);
    assert!(err.is_null());
    assert!((22000..22100).contains(&port_out));
}

#[test]
fn arena_find_available_port_linear_returns_port_via_out_param() {
    let mut port_out: i32 = 0;
    let mut err: *mut c_char = std::ptr::null_mut();

    let status = arena_find_available_port(22200, 22300, 1, &mut port_out, &mut err);

    assert_eq!(status, ArenaStatus::Ok);
    assert!(err.is_null());
    assert!((22200..22300).contains(&port_out));
}

#[test]
fn arena_find_available_port_null_port_out_returns_invalid_argument() {
    let mut err: *mut c_char = std::ptr::null_mut();

    let status = arena_find_available_port(22400, 22500, 0, std::ptr::null_mut(), &mut err);

    assert_eq!(status, ArenaStatus::InvalidArgument);
    assert!(!err.is_null());
    arena_free_string(err);
}

#[test]
fn arena_find_available_port_inverted_range_returns_invalid_argument() {
    let mut port_out: i32 = 0;
    let mut err: *mut c_char = std::ptr::null_mut();

    let status = arena_find_available_port(500, 500, 0, &mut port_out, &mut err);

    assert_eq!(status, ArenaStatus::InvalidArgument);
    assert!(!err.is_null());
    arena_free_string(err);
}

#[test]
fn arena_find_available_port_out_of_bounds_range_returns_invalid_argument() {
    let mut port_out: i32 = 0;
    let mut err: *mut c_char = std::ptr::null_mut();

    let status = arena_find_available_port(-1, 100, 0, &mut port_out, &mut err);

    assert_eq!(status, ArenaStatus::InvalidArgument);
    assert!(!err.is_null());
    arena_free_string(err);
}

#[test]
fn arena_find_available_port_unrecognized_strategy_returns_invalid_argument() {
    let mut port_out: i32 = 0;
    let mut err: *mut c_char = std::ptr::null_mut();

    let status = arena_find_available_port(22600, 22700, 99, &mut port_out, &mut err);

    assert_eq!(status, ArenaStatus::InvalidArgument);
    assert!(!err.is_null());
    arena_free_string(err);
}

#[test]
fn arena_find_available_port_range_end_65536_reaches_port_65535() {
    let held = TcpListener::bind(("127.0.0.1", 65534)).expect("bind held port");

    let mut port_out: i32 = 0;
    let mut err: *mut c_char = std::ptr::null_mut();
    let status = arena_find_available_port(65534, 65536, 1, &mut port_out, &mut err);

    assert_eq!(status, ArenaStatus::Ok);
    assert!(err.is_null());
    assert_eq!(port_out, 65535);

    drop(held);
}

#[test]
fn arena_find_available_port_range_end_above_65536_returns_invalid_argument() {
    let mut port_out: i32 = 0;
    let mut err: *mut c_char = std::ptr::null_mut();

    let status = arena_find_available_port(0, 65537, 0, &mut port_out, &mut err);

    assert_eq!(status, ArenaStatus::InvalidArgument);
    assert!(!err.is_null());
    arena_free_string(err);
}

#[test]
fn arena_find_available_port_no_port_available_returns_panic_status() {
    let range_start = 22800i32;
    let range_end = 22802i32;
    let held: Vec<TcpListener> = (range_start..range_end)
        .map(|p| TcpListener::bind(("127.0.0.1", p as u16)).expect("bind held port"))
        .collect();

    let mut port_out: i32 = 0;
    let mut err: *mut c_char = std::ptr::null_mut();
    let status = arena_find_available_port(range_start, range_end, 1, &mut port_out, &mut err);

    assert_eq!(status, ArenaStatus::Panic);
    assert!(!err.is_null());
    let msg = unsafe { CStr::from_ptr(err) }.to_string_lossy().into_owned();
    assert!(msg.contains("no available port found"));
    arena_free_string(err);

    drop(held);
}
