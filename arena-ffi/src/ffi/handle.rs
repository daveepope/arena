use std::sync::Mutex;

use arena::OpenArena;
use tokio::runtime::Runtime;

#[repr(C)]
pub struct ArenaHandle {
    _private: [u8; 0],
}

pub(crate) struct HandleInner {
    pub runtime: Runtime,
    pub state: Mutex<Option<OpenArena>>,
}

impl HandleInner {
    pub fn new(runtime: Runtime, arena: OpenArena) -> Self {
        Self {
            runtime,
            state: Mutex::new(Some(arena)),
        }
    }

    pub fn into_raw(self) -> *mut ArenaHandle {
        Box::into_raw(Box::new(self)) as *mut ArenaHandle
    }

    pub unsafe fn from_raw(ptr: *mut ArenaHandle) -> Box<HandleInner> {
        unsafe { Box::from_raw(ptr as *mut HandleInner) }
    }

    pub unsafe fn as_ref<'a>(ptr: *mut ArenaHandle) -> &'a HandleInner {
        unsafe { &*(ptr as *const HandleInner) }
    }
}
