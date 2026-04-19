use crate::mssql_dependency::MssqlDependency;
use crate::mssql_dependency::mssql_container_impl::connect;
use async_trait::async_trait;

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
            identifier: dependency.identifier.clone(),
            managed_tables: dependency.managed_tables().to_vec(),
        }
    }

    pub async fn run(self) -> ActivePlaybook {
        reset_tables(&self.identifier, &self.connection_string, &self.managed_tables)
            .await
            .unwrap_or_else(|e| panic!("{e}"));

        log::info!(
            "[MssqlPlaybook-{}] reset {} managed table(s); state is now clean.",
            self.identifier,
            self.managed_tables.len()
        );

        ActivePlaybook {
            connection_string: self.connection_string,
            identifier: self.identifier,
            managed_tables: self.managed_tables,
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
                log::warn!(
                    "[MssqlPlaybook-{identifier}] drop: failed to build runtime: {e}"
                );
                return Ok(());
            }
        };
        rt.block_on(async move {
            reset_tables(&identifier, &connection_string, &managed_tables).await
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

#[async_trait]
impl arena::Playbook for Playbook {
    type Active = ActivePlaybook;

    async fn run(self) -> Self::Active {
        Playbook::run(self).await
    }
}

pub struct ActivePlaybook {
    connection_string: String,
    identifier: String,
    managed_tables: Vec<(String, String)>,
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
        log::debug!(
            "[MssqlPlaybook-{identifier}] reset: no managed tables; nothing to do."
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
        sql.push_str(&format!(
            "DELETE FROM {};\n",
            quote_ident(schema, name)
        ));
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

fn quote_ident(schema: &str, name: &str) -> String {
    format!(
        "[{}].[{}]",
        schema.replace(']', "]]"),
        name.replace(']', "]]")
    )
}
