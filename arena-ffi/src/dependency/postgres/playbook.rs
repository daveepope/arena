use std::os::raw::c_char;

use arena_postgres::ActivePlaybook;
use async_trait::async_trait;

use crate::active_playbook::ArenaActivePlaybookHandle;
use crate::dependency::playbook_dispatch::{verify_playbook_query, PlaybookQueryVerify};
use crate::ArenaStatus;

#[async_trait]
impl PlaybookQueryVerify for ActivePlaybook {
    async fn verify_query(&self, query: &str) -> i32 {
        self.verify(query).await
    }
}

#[no_mangle]
pub extern "C" fn arena_postgres_playbook_verify(
    handle: *mut ArenaActivePlaybookHandle,
    verify_spec: *const c_char,
    err_out: *mut *mut c_char,
) -> ArenaStatus {
    verify_playbook_query::<ActivePlaybook>(
        handle,
        verify_spec,
        err_out,
        "arena_postgres_playbook_verify",
        "Postgres",
    )
}
