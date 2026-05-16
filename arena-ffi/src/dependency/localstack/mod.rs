pub(crate) mod localstack_dependency;
pub mod playbook;

pub use playbook::{
    arena_localstack_playbook_close, arena_localstack_playbook_open, ArenaLocalstackPlaybookHandle,
};
