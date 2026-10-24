use std::ffi::CString;
use std::os::raw::c_char;
use std::path::Path;
use std::sync::Arc;

use arena::lifecycle::ArenaLifecycleObserver;
use arena_dashboard::HttpForwardingObserver;

use crate::boundary::call_across_boundary;
use crate::error::{clear_error, write_error};
use crate::panic_payload::panic_message;
use crate::strings::c_str_to_string;

#[no_mangle]
pub extern "C" fn arena_dashboard_ensure_running(
    port: u16,
    search_dir_utf8: *const c_char,
    err_out: *mut *mut c_char,
) -> *mut c_char {
    unsafe { clear_error(err_out) };
    let search_dir = unsafe { c_str_to_string(search_dir_utf8) };

    let outcome = call_across_boundary(move || {
        let search_dir_path = search_dir.as_deref().map(Path::new);
        arena_dashboard::ensure_running(port, search_dir_path)
    });

    match outcome {
        Ok(Ok(url)) => match CString::new(url) {
            Ok(c) => c.into_raw(),
            Err(_) => {
                unsafe {
                    write_error(
                        err_out,
                        "arena_dashboard_ensure_running: url contained interior NUL",
                    )
                };
                std::ptr::null_mut()
            }
        },
        Ok(Err(e)) => {
            unsafe { write_error(err_out, e.to_string()) };
            std::ptr::null_mut()
        }
        Err(payload) => {
            let msg = panic_message(payload.as_ref());
            unsafe {
                write_error(err_out, format!("arena_dashboard_ensure_running failed: {msg}"))
            };
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn arena_register_dashboard_observer(dashboard_url_utf8: *const c_char) -> u64 {
    let Some(url) = (unsafe { c_str_to_string(dashboard_url_utf8) }) else {
        return 0;
    };
    call_across_boundary(move || {
        let forwarder = Arc::new(HttpForwardingObserver::new(url));
        let recipient: Arc<dyn ArenaLifecycleObserver> = forwarder.clone();
        let token = crate::lifecycle_observer::register_observer(recipient);
        if token == 0 {
            return 0;
        }
        crate::lifecycle_observer::register_playbook_observer(
            token,
            arena_dashboard::PlaybookForwarder::new(Arc::clone(&forwarder)),
        );
        let profile_token = arena_profile::register_observer(
            forwarder as Arc<dyn arena_profile::CpuProfileObserver>,
        );
        crate::lifecycle_observer::pair_cpu_profile_observer(token, profile_token);
        token
    })
    .unwrap_or(0)
}
