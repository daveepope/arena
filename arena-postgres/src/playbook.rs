use crate::blocking::run_blocking;
use crate::postgres_dependency::PostgresDependency;

pub struct Playbook {
    connection_string: String,
    identifier: String,
    managed_tables: Vec<(String, String)>,
}

impl Playbook {
    pub fn with(dependency: &PostgresDependency) -> Self {
        let connection_string = dependency
            .connection_string()
            .expect("PostgresDependency must be started before configuring a Playbook")
            .to_string();
        Self {
            connection_string,
            identifier: format!("postgres-playbook:{}", dependency.identifier),
            managed_tables: dependency.managed_tables().to_vec(),
        }
    }

    pub fn with_identifier(mut self, identifier: impl Into<String>) -> Self {
        self.identifier = identifier.into();
        self
    }

    pub async fn run(self) -> ActivePlaybook {
        let identifier = self.identifier.clone();
        let connection_string = self.connection_string.clone();
        let managed_tables = self.managed_tables.clone();

        let tables = run_blocking(move || {
            let mut client = postgres::Client::connect(&connection_string, postgres::NoTls)
                .unwrap_or_else(|e| panic!("[PostgresPlaybook-{identifier}] connect failed: {e}"));

            let tables = if managed_tables.is_empty() {
                discover_user_tables(&identifier, &mut client).unwrap_or_else(|e| panic!("{e}"))
            } else {
                managed_tables
            };

            reset_tables(&identifier, &mut client, &tables).unwrap_or_else(|e| panic!("{e}"));

            tables
        })
        .await;

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

fn reset_on_drop(identifier: String, connection_string: String, managed_tables: Vec<(String, String)>) {
    let already_unwinding = std::thread::panicking();

    let handle = std::thread::spawn(move || {
        let mut client = postgres::Client::connect(&connection_string, postgres::NoTls)
            .map_err(|e| format!("[PostgresPlaybook-{identifier}] drop: connect failed: {e}"))?;
        reset_tables(&identifier, &mut client, &managed_tables)
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
        let identifier = self.identifier.clone();
        let connection_string = self.connection_string.clone();
        let query = query.to_string();

        run_blocking(move || {
            let mut client =
                postgres::Client::connect(&connection_string, postgres::NoTls).unwrap_or_else(
                    |e| panic!("[PostgresPlaybook-{identifier}] verify: connect failed: {e}"),
                );

            let rows = client.query(query.as_str(), &[]).unwrap_or_else(|e| {
                panic!("[PostgresPlaybook-{identifier}] verify: query {query:?} failed: {e}")
            });

            let row = rows.first().unwrap_or_else(|| {
                panic!(
                    "[PostgresPlaybook-{identifier}] verify: query {query:?} returned no rows"
                )
            });

            match row.try_get::<_, i32>(0) {
                Ok(v) => v,
                Err(_) => {
                    let as_i64: i64 = row.try_get(0).unwrap_or_else(|e| {
                        panic!(
                            "[PostgresPlaybook-{identifier}] verify: query {query:?} first column is not an integer: {e}"
                        )
                    });
                    i32::try_from(as_i64).unwrap_or_else(|_| {
                        panic!(
                            "[PostgresPlaybook-{identifier}] verify: query {query:?} first column ({as_i64}) does not fit in i32"
                        )
                    })
                }
            }
        })
        .await
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

fn reset_tables(
    identifier: &str,
    client: &mut postgres::Client,
    tables: &[(String, String)],
) -> Result<(), String> {
    if tables.is_empty() {
        tracing::debug!(
            playbook_id = %identifier,
            "reset skipped: no managed tables"
        );
        return Ok(());
    }

    let mut sql = String::new();

    for (schema, name) in tables {
        sql.push_str(&format!(
            "ALTER TABLE {} DISABLE TRIGGER ALL;\n",
            quote_ident(schema, name)
        ));
    }
    for (schema, name) in tables {
        sql.push_str(&format!("DELETE FROM {};\n", quote_ident(schema, name)));
    }
    for (schema, name) in tables {
        sql.push_str(&format!(
            "ALTER TABLE {} ENABLE TRIGGER ALL;\n",
            quote_ident(schema, name)
        ));
    }

    client
        .batch_execute(sql.as_str())
        .map_err(|e| format!("[PostgresPlaybook-{identifier}] reset: delete failed: {e}"))?;

    Ok(())
}

fn discover_user_tables(
    identifier: &str,
    client: &mut postgres::Client,
) -> Result<Vec<(String, String)>, String> {
    let sql = "SELECT schemaname, tablename \
               FROM pg_catalog.pg_tables \
               WHERE schemaname NOT IN ('pg_catalog', 'information_schema') \
               ORDER BY schemaname, tablename;";

    let rows = client.query(sql, &[]).map_err(|e| {
        format!("[PostgresPlaybook-{identifier}] discover_user_tables: query failed: {e}")
    })?;

    let mut tables = Vec::with_capacity(rows.len());
    for row in rows {
        let schema: &str = row.try_get(0).unwrap_or("");
        let name: &str = row.try_get(1).unwrap_or("");
        if !schema.is_empty() && !name.is_empty() {
            tables.push((schema.to_string(), name.to_string()));
        }
    }
    Ok(tables)
}

fn quote_ident(schema: &str, name: &str) -> String {
    format!(
        "\"{}\".\"{}\"",
        schema.replace('"', "\"\""),
        name.replace('"', "\"\"")
    )
}
