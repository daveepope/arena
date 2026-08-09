#![no_main]

use arena_http::{get_requested_for, RecordedRequest};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    if let Ok(request) = serde_json::from_str::<RecordedRequest>(text) {
        let criteria = get_requested_for(&request.url);
        let _ = criteria.method_and_path();
    }
});
