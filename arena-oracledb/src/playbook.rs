use crate::oracle_dependency::oracle_container_impl::{self, OracleImpl};
use crate::oracle_dependency::OracleDependency;
use std::sync::Arc;

pub struct Playbook {
    oracle_impl: Arc<dyn OracleImpl>,
    username: String,
    password: String,
    identifier: String,
    managed_tables: Vec<String>,
}

impl Playbook {
    pub fn with(dependency: &OracleDependency) -> Self {
        dependency
            .connection_string()
            .expect("OracleDependency must be started before configuring a Playbook");

        Self {
            oracle_impl: dependency.oracle_impl(),
            username: dependency.database_username().to_string(),
            password: dependency.database_password().to_string(),
            identifier: format!("oracle-playbook:{}", dependency.identifier),
            managed_tables: dependency.managed_tables().to_vec(),
        }
    }

    pub fn with_identifier(mut self, identifier: impl Into<String>) -> Self {
        self.identifier = identifier.into();
        self
    }

    pub async fn run(self) -> ActivePlaybook {
        let tables = if self.managed_tables.is_empty() {
            discover_user_tables(self.oracle_impl.as_ref(), &self.username, &self.password, &self.identifier)
                .await
                .unwrap_or_else(|e| panic!("{e}"))
        } else {
            self.managed_tables
        };

        reset_tables(
            self.oracle_impl.as_ref(),
            &self.username,
            &self.password,
            &self.identifier,
            &tables,
        )
        .await
        .unwrap_or_else(|e| panic!("{e}"));

        tracing::debug!(
            playbook_id = %self.identifier,
            table_count = tables.len(),
            "reset tables; clean state"
        );

        ActivePlaybook {
            oracle_impl: self.oracle_impl,
            username: self.username,
            password: self.password,
            identifier: self.identifier,
            managed_tables: tables,
        }
    }
}

fn reset_on_drop(
    oracle_impl: Arc<dyn OracleImpl>,
    identifier: String,
    username: String,
    password: String,
    managed_tables: Vec<String>,
) {
    let already_unwinding = std::thread::panicking();

    let handle = std::thread::spawn(move || -> Result<(), String> {
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                tracing::warn!(
                    playbook_id = %identifier,
                    error = %e,
                    "drop cleanup: runtime build failed"
                );
                return Ok(());
            }
        };
        rt.block_on(async move {
            reset_tables(
                oracle_impl.as_ref(),
                &username,
                &password,
                &identifier,
                &managed_tables,
            )
            .await
        })
    });

    let outcome = handle.join();

    if already_unwinding {
        return;
    }

    match outcome {
        Ok(Ok(())) => {}
        Ok(Err(msg)) => panic!("{msg}"),
        Err(_) => panic!("ActivePlaybook::drop: cleanup thread panicked"),
    }
}

pub struct ActivePlaybook {
    oracle_impl: Arc<dyn OracleImpl>,
    username: String,
    password: String,
    identifier: String,
    managed_tables: Vec<String>,
}

impl arena::ActivePlaybook for ActivePlaybook {
    fn identifier(&self) -> &str {
        &self.identifier
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl ActivePlaybook {
    pub fn identifier(&self) -> &str {
        &self.identifier
    }

    pub async fn verify(&self, query: &str) -> i32 {
        oracle_container_impl::exec_scalar_query(
            self.oracle_impl.as_ref(),
            &self.username,
            &self.password,
            query,
        )
        .await
        .unwrap_or_else(|e| panic!("[OraclePlaybook-{}] verify: {e}", self.identifier))
    }
}

impl Drop for ActivePlaybook {
    fn drop(&mut self) {
        let identifier = std::mem::take(&mut self.identifier);
        let username = std::mem::take(&mut self.username);
        let password = std::mem::take(&mut self.password);
        let managed_tables = std::mem::take(&mut self.managed_tables);

        if identifier.is_empty() {
            return;
        }

        reset_on_drop(
            Arc::clone(&self.oracle_impl),
            identifier,
            username,
            password,
            managed_tables,
        );
    }
}

fn quote_ident(ident: &str) -> String {
    format!("\"{}\"", ident.replace('"', "\"\""))
}

fn build_constraint_sql(constraints: &[(String, String)], action: &str) -> String {
    constraints
        .iter()
        .map(|(table, constraint)| {
            format!(
                "ALTER TABLE {} {action} CONSTRAINT {};\n",
                quote_ident(table),
                quote_ident(constraint)
            )
        })
        .collect()
}

async fn reset_tables(
    oracle_impl: &dyn OracleImpl,
    username: &str,
    password: &str,
    identifier: &str,
    tables: &[String],
) -> Result<(), String> {
    if tables.is_empty() {
        tracing::debug!(
            playbook_id = %identifier,
            "reset skipped: no managed tables"
        );
        return Ok(());
    }

    let constraints = discover_foreign_keys(oracle_impl, username, password, identifier, tables).await?;

    if !constraints.is_empty() {
        let disable_sql = build_constraint_sql(&constraints, "DISABLE");
        oracle_container_impl::exec_sql(oracle_impl, username, password, &disable_sql)
            .await
            .map_err(|e| format!("[OraclePlaybook-{identifier}] reset: disable constraints: {e}"))?;
    }

    let delete_sql: String = tables
        .iter()
        .map(|table| format!("DELETE FROM {};\n", quote_ident(table)))
        .collect();
    let delete_result = oracle_container_impl::exec_sql(oracle_impl, username, password, &delete_sql).await;

    if !constraints.is_empty() {
        let enable_sql = build_constraint_sql(&constraints, "ENABLE");
        if let Err(e) = oracle_container_impl::exec_sql(oracle_impl, username, password, &enable_sql).await {
            tracing::warn!(
                playbook_id = %identifier,
                error = %e,
                "reset: failed to re-enable constraints after reset"
            );
            if delete_result.is_ok() {
                return Err(format!("[OraclePlaybook-{identifier}] reset: enable constraints: {e}"));
            }
        }
    }

    delete_result.map_err(|e| format!("[OraclePlaybook-{identifier}] reset: delete: {e}"))?;

    Ok(())
}

async fn discover_user_tables(
    oracle_impl: &dyn OracleImpl,
    username: &str,
    password: &str,
    identifier: &str,
) -> Result<Vec<String>, String> {
    oracle_container_impl::exec_table_list(
        oracle_impl,
        username,
        password,
        "SELECT TABLE_NAME FROM USER_TABLES ORDER BY TABLE_NAME;",
    )
    .await
    .map_err(|e| format!("[OraclePlaybook-{identifier}] discover_user_tables: {e}"))
}

async fn discover_foreign_keys(
    oracle_impl: &dyn OracleImpl,
    username: &str,
    password: &str,
    identifier: &str,
    tables: &[String],
) -> Result<Vec<(String, String)>, String> {
    if tables.is_empty() {
        return Ok(Vec::new());
    }

    let table_list = tables
        .iter()
        .map(|t| format!("'{}'", t.replace('\'', "''")))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "SELECT table_name || '|' || constraint_name FROM user_constraints \
         WHERE constraint_type = 'R' AND table_name IN ({table_list});"
    );

    oracle_container_impl::exec_constraint_list(oracle_impl, username, password, &sql)
        .await
        .map_err(|e| format!("[OraclePlaybook-{identifier}] discover_foreign_keys: {e}"))
}
