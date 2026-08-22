pub mod dependency;

mod active_playbook;
mod closed_arena;
mod dependency_reset;
mod error;
mod logging;
mod loopback_tls;
mod open_arena;
mod panic_payload;
pub(crate) mod strings;

pub mod containerized_component;
mod executable_component;
pub mod healthcheck;
mod kafka_dependency;
mod managed_playbook;
mod matches;
mod postgres_dependency;
pub mod runtime_args;

pub use active_playbook::{
    arena_active_playbook_drop, arena_match_playbook_run, ArenaActivePlaybookHandle,
};
pub use closed_arena::{arena_open, OpenArenaHandle};
#[cfg(feature = "bench-support")]
pub use closed_arena::parse_config_for_bench;
pub use dependency_reset::{arena_hard_reset, arena_soft_reset};
pub use error::ArenaStatus;
pub use logging::{
    arena_add_log_target, arena_dispatcher_default_logging_target_logger_name_utf8,
    arena_dispatcher_default_logging_target_publish_level,
    arena_dispatcher_component_allow_json_set, arena_dispatcher_dependency_allow_json_set,
    arena_remove_log_target, arena_set_log_level,
    ArenaLogCallback, ArenaLogLevel,
};
pub use open_arena::arena_close;
pub use strings::arena_free_string;
pub use dependency::http::{arena_http_playbook_open, arena_http_playbook_verify};
pub use dependency::mssql::arena_mssql_playbook_verify;
pub use dependency::postgres::arena_postgres_playbook_verify;
