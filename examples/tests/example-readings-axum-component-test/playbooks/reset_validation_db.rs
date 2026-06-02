use arena_mssql::ManagedMssqlPlaybook;

use super::ids::reset_validation_db_id;

pub fn reset_validation_db_playbook(mssql_id: impl Into<String>) -> ManagedMssqlPlaybook {
    ManagedMssqlPlaybook::new(reset_validation_db_id(), mssql_id)
}
