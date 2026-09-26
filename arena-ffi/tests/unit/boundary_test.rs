use arena_ffi::boundary::{call_across_boundary, inside_boundary};
use arena_ffi::panic_payload::panic_message;

#[test]
fn inside_boundary_outside_any_call_returns_false() {
    assert!(!inside_boundary());
}

#[test]
fn call_across_boundary_running_body_reports_inside_boundary() {
    let observed = call_across_boundary(inside_boundary).expect("body must not panic");

    assert!(observed);
}

#[test]
fn call_across_boundary_successful_body_returns_its_value() {
    let outcome = call_across_boundary(|| 7_u32).expect("body must not panic");

    assert_eq!(outcome, 7);
}

#[test]
fn call_across_boundary_panicking_body_returns_the_payload() {
    let outcome = call_across_boundary(|| panic!("boundary-payload"));

    let payload = outcome.expect_err("panic must be captured");
    assert_eq!(
        panic_message(payload.as_ref()),
        "boundary-payload"
    );
}

#[test]
fn call_across_boundary_panicking_body_leaves_the_boundary() {
    let _ = call_across_boundary(|| panic!("boundary-unwind"));

    assert!(!inside_boundary());
}

#[test]
fn call_across_boundary_nested_calls_stay_inside_until_the_outermost_returns() {
    let observed = call_across_boundary(|| {
        let inner = call_across_boundary(inside_boundary).expect("inner body must not panic");
        (inner, inside_boundary())
    })
    .expect("outer body must not panic");

    assert_eq!(observed, (true, true));
    assert!(!inside_boundary());
}
