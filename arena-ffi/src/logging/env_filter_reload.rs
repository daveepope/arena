use std::sync::{Mutex, OnceLock};

use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::reload;

use super::severity_level::Level;

pub(crate) trait FilterControl: Send + Sync {
    fn reload(&self, filter: EnvFilter) -> Result<(), String>;
}

impl<S> FilterControl for reload::Handle<EnvFilter, S>
where
    S: tracing::Subscriber + Send + Sync + 'static,
{
    fn reload(&self, filter: EnvFilter) -> Result<(), String> {
        self.reload(filter).map_err(|e| e.to_string())
    }
}

struct State {
    control: Box<dyn FilterControl>,
    global: Mutex<Level>,
}

static STATE: OnceLock<State> = OnceLock::new();

pub(crate) fn install_filter_control(control: Box<dyn FilterControl>, initial: Level) {
    let _ = STATE.get_or_init(|| State {
        control,
        global: Mutex::new(initial),
    });
}

pub(crate) fn set_global_level(level: Level) {
    if let Some(state) = STATE.get() {
        *state.global.lock().unwrap_or_else(|e| e.into_inner()) = level;
        rebuild(state);
    }
}

fn rebuild(state: &State) {
    let global = *state.global.lock().unwrap_or_else(|e| e.into_inner());
    let directives = global.directive();
    if let Ok(filter) = EnvFilter::try_new(directives) {
        let _ = state.control.reload(filter);
    }
}
