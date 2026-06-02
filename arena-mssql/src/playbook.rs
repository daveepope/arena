use crate::mssql_dependency::mssql_container_impl::connect;
use crate::mssql_dependency::MssqlDependency;

pub struct Playbook {
    connection_string: String,
    identifier: String,
    managed_tables: Vec<(String, String)>,
}

impl Playbook {
    pub fn with(dependency: &MssqlDependency) -> Self {
        let connection_string = dependency
            .connection_string()
            .expect("MssqlDependency must be started before configuring a Playbook")
            .to_string();
        Self {
            connection_string,
            identifier: format!("mssql-playbook:{}", dependency.identifier),
            managed_tables: dependency.managed_tables().to_vec(),
        }
    }

    pub fn with_identifier(mut self, identifier: impl Into<String>) -> Self {
        self.identifier = identifier.into();
        self
    }

    pub async fn run(self) -> ActivePlaybook {
        let tables = if self.managed_tables.is_empty() {
            discover_user_tables(&self.identifier, &self.connection_string)
                .await
                .unwrap_or_else(|e| panic!("{e}"))
        } else {
            self.managed_tables
        };

        reset_tables(&self.identifier, &self.connection_string, &tables)
            .await
            .unwrap_or_else(|e| panic!("{e}"));

        tracing::debug!(
            playbook_id = %self.identifier,
            table_count = tables.len(),
            "reset tables; clean state"
        );

        ActivePlaybook {
            connection_string: self.connection_string,
            identifier: self.identifier,
            managed_tables: tables,
        }
    }
}

fn reset_on_drop(
    identifier: String,
    connection_string: String,
    managed_tables: Vec<(String, String)>,
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
        rt.block_on(
            async move { reset_tables(&identifier, &connection_string, &managed_tables).await },
        )
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
    connection_string: String,
    identifier: String,
    managed_tables: Vec<(String, String)>,
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
    pub fn connection_string(&self) -> &str {
        &self.connection_string
    }

    pub fn identifier(&self) -> &str {
        &self.identifier
    }

    pub async fn verify(&self, query: &str) -> i32 {
        let mut client = connect(&self.connection_string).await.unwrap_or_else(|e| {
            panic!(
                "[MssqlPlaybook-{}] verify: connect failed: {e}",
                self.identifier
            )
        });

        let stream = client.simple_query(query).await.unwrap_or_else(|e| {
            panic!(
                "[MssqlPlaybook-{}] verify: query {query:?} failed: {e}",
                self.identifier
            )
        });

        let row = stream.into_row().await.unwrap_or_else(|e| {
            panic!(
                "[MssqlPlaybook-{}] verify: read row for {query:?} failed: {e}",
                self.identifier
            )
        });

        let row = row.unwrap_or_else(|| {
            panic!(
                "[MssqlPlaybook-{}] verify: query {query:?} returned no rows",
                self.identifier
            )
        });

        row.get::<i32, _>(0).unwrap_or_else(|| {
            panic!(
                "[MssqlPlaybook-{}] verify: query {query:?} first column is not an i32",
                self.identifier
            )
        })
    }
}

impl Drop for ActivePlaybook {
    fn drop(&mut self) {
        let identifier = std::mem::take(&mut self.identifier);
        let connection_string = std::mem::take(&mut self.connection_string);
        let managed_tables = std::mem::take(&mut self.managed_tables);

        if connection_string.is_empty() {
            return;
        }

        reset_on_drop(identifier, connection_string, managed_tables);
    }
}

async fn reset_tables(
    identifier: &str,
    connection_string: &str,
    tables: &[(String, String)],
) -> Result<(), String> {
    if tables.is_empty() {
        tracing::debug!(
            playbook_id = %identifier,
            "reset skipped: no managed tables"
        );
        return Ok(());
    }

    let mut client = connect(connection_string)
        .await
        .map_err(|e| format!("[MssqlPlaybook-{identifier}] reset: connect failed: {e}"))?;

    let mut sql = String::new();

    for (schema, name) in tables {
        sql.push_str(&format!(
            "ALTER TABLE {} NOCHECK CONSTRAINT ALL;\n",
            quote_ident(schema, name)
        ));
    }
    for (schema, name) in tables {
        sql.push_str(&format!("DELETE FROM {};\n", quote_ident(schema, name)));
    }
    for (schema, name) in tables {
        sql.push_str(&format!(
            "ALTER TABLE {} WITH CHECK CHECK CONSTRAINT ALL;\n",
            quote_ident(schema, name)
        ));
    }

    client
        .simple_query(sql.as_str())
        .await
        .map_err(|e| format!("[MssqlPlaybook-{identifier}] reset: delete failed: {e}"))?;

    Ok(())
}

async fn discover_user_tables(
    identifier: &str,
    connection_string: &str,
) -> Result<Vec<(String, String)>, String> {
    let mut client = connect(connection_string).await.map_err(|e| {
        format!("[MssqlPlaybook-{identifier}] discover_user_tables: connect failed: {e}")
    })?;

    let sql = "SELECT s.name AS schema_name, t.name AS table_name \
               FROM sys.tables t \
               INNER JOIN sys.schemas s ON t.schema_id = s.schema_id \
               WHERE t.is_ms_shipped = 0 \
               ORDER BY s.name, t.name;";

    let stream = client.simple_query(sql).await.map_err(|e| {
        format!("[MssqlPlaybook-{identifier}] discover_user_tables: query failed: {e}")
    })?;

    let rows = stream.into_first_result().await.map_err(|e| {
        format!("[MssqlPlaybook-{identifier}] discover_user_tables: read failed: {e}")
    })?;

    let mut tables = Vec::with_capacity(rows.len());
    for row in rows {
        let schema: &str = row.get::<&str, _>(0).unwrap_or("");
        let name: &str = row.get::<&str, _>(1).unwrap_or("");
        if !schema.is_empty() && !name.is_empty() {
            tables.push((schema.to_string(), name.to_string()));
        }
    }
    Ok(tables)
}

fn quote_ident(schema: &str, name: &str) -> String {
    format!(
        "[{}].[{}]",
        schema.replace(']', "]]"),
        name.replace(']', "]]")
    )
}
