use std::os::raw::c_char;

use arena_host::find_available_port::{
    find_available_port as host_find_available_port, PortSearchStrategy,
};

use crate::error::{clear_error, write_error, ArenaStatus};
use crate::panic_payload::panic_message;
use crate::boundary::call_across_boundary;

#[no_mangle]
pub extern "C" fn arena_find_available_port(
    range_start: i32,
    range_end: i32,
    strategy: i32,
    port_out: *mut i32,
    err_out: *mut *mut c_char,
) -> ArenaStatus {
    unsafe { clear_error(err_out) };

    if port_out.is_null() {
        unsafe { write_error(err_out, "arena_find_available_port: port_out must not be null") };
        return ArenaStatus::InvalidArgument;
    }
    const MAX_RANGE_END: i32 = u16::MAX as i32 + 1;
    if !(0..=u16::MAX as i32).contains(&range_start) || !(1..=MAX_RANGE_END).contains(&range_end) {
        unsafe {
            write_error(
                err_out,
                format!(
                    "arena_find_available_port: range_start ({range_start}) must be within 0..=65535 and range_end ({range_end}) must be within 1..=65536"
                ),
            )
        };
        return ArenaStatus::InvalidArgument;
    }
    if range_start >= range_end {
        unsafe {
            write_error(
                err_out,
                format!(
                    "arena_find_available_port: range_start ({range_start}) must be < range_end ({range_end})"
                ),
            )
        };
        return ArenaStatus::InvalidArgument;
    }
    let strategy = match strategy {
        0 => PortSearchStrategy::Random,
        1 => PortSearchStrategy::Linear,
        other => {
            unsafe {
                write_error(
                    err_out,
                    format!("arena_find_available_port: unrecognized strategy {other}"),
                )
            };
            return ArenaStatus::InvalidArgument;
        }
    };
    let range_start = range_start as u16;
    let range_end_inclusive = (range_end - 1) as u16;

    let outcome = call_across_boundary(|| {
        host_find_available_port(range_start..=range_end_inclusive, strategy).unwrap_or_else(|| {
            panic!("no available port found in range {range_start}..{range_end}")
        })
    });

    match outcome {
        Ok(port) => {
            unsafe { *port_out = port as i32 };
            ArenaStatus::Ok
        }
        Err(payload) => {
            let msg = panic_message(payload.as_ref());
            tracing::error!(
                panic_message = %msg,
                op = "arena_find_available_port",
                "panic while finding an available port"
            );
            unsafe { write_error(err_out, format!("arena_find_available_port failed: {msg}")) };
            ArenaStatus::Panic
        }
    }
}
