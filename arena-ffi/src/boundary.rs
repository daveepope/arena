use std::cell::Cell;
use std::panic::AssertUnwindSafe;

thread_local! {
    static BOUNDARY_DEPTH: Cell<u32> = const { Cell::new(0) };
}

pub fn inside_boundary() -> bool {
    BOUNDARY_DEPTH.with(|depth| depth.get() > 0)
}

struct BoundaryScope;

impl BoundaryScope {
    fn enter() -> Self {
        BOUNDARY_DEPTH.with(|depth| depth.set(depth.get().saturating_add(1)));
        Self
    }
}

impl Drop for BoundaryScope {
    fn drop(&mut self) {
        BOUNDARY_DEPTH.with(|depth| depth.set(depth.get().saturating_sub(1)));
    }
}

pub fn call_across_boundary<F, T>(op: F) -> std::thread::Result<T>
where
    F: FnOnce() -> T,
{
    let _scope = BoundaryScope::enter();
    std::panic::catch_unwind(AssertUnwindSafe(op))
}
