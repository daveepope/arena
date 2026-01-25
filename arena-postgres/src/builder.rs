use arena::dependency::RunnableDependency;
use crate::postgres_container_impl::{PostgresContainerImpl, PostgresImpl};
use crate::postgres_dependency::{PostgresDependency};

pub struct PostgresDependencyBuilder {
    identifier: String,
    postgres_impl: Option<Box<dyn PostgresImpl>>,
    port: Option<u16>,
    database_name: Option<String>,
    database_username: Option<String>,
    database_password: Option<String>,
    startup_sql_scripts: Option<Vec<String>>,
    dependencies: Option<Vec<Box<dyn RunnableDependency>>>,
    image_tag: Option<String>,
    container_name: Option<String>,
}

impl PostgresDependencyBuilder {

    const DEFAULT_PORT: u16 = 5432;
    const DEFAULT_DATABASE_NAME: &'static str = "arena_db";
    const DEFAULT_DATABASE_USERNAME: &'static str = "arena_user";
    const DEFAULT_DATABASE_PASSWORD: &'static str = "postgres";
    const DEFAULT_IMAGE_TAG: &'static str = "latest";

    pub(crate) fn new(identifier: impl Into<String>) -> Self {
        Self {
            identifier: identifier.into(),
            postgres_impl: None,
            port: None,
            database_name: None,
            database_username: None,
            database_password: None,
            startup_sql_scripts: None,
            dependencies: None,
            image_tag: None,
            container_name: None,
        }
    }

    pub fn with_impl<W>(mut self, wrapper: W) -> Self
    where
        W: PostgresImpl + 'static,
    {
        self.postgres_impl = Some(Box::new(wrapper));
        self
    }

    pub fn with_port(mut self, port: u16) -> Self
    {
        self.port = Option::from(port);
        self
    }

    pub fn with_database_name(mut self, database_name: impl Into<String>) -> Self
    {
        self.database_name = Option::from(database_name.into());
        self
    }

    pub fn with_database_username(mut self, database_username: impl Into<String>) -> Self
    {
        self.database_username = Option::from(database_username.into());
        self
    }

    pub fn with_database_password(mut self, database_password: impl Into<String>) -> Self
    {
        self.database_password = Option::from(database_password.into());
        self
    }

    pub fn with_startup_sql_scripts(mut self, scripts: Vec<String>) -> Self
    {
        self.startup_sql_scripts = Option::from(scripts);
        self
    }

    pub fn with_child_dependencies(mut self, dependencies: Vec<Box<dyn RunnableDependency>>) -> Self
    {
        self.dependencies = Option::from(dependencies);
        self
    }

    pub fn with_image_tag(mut self, image_tag: impl Into<String>) -> Self {
        self.image_tag = Some(image_tag.into());
        self
    }

    // Convenience alias.
    pub fn with_image(self, image_tag: impl Into<String>) -> Self {
        self.with_image_tag(image_tag)
    }

    pub fn with_container_name(mut self, container_name: impl Into<String>) -> Self {
        self.container_name = Some(container_name.into());
        self
    }

    // Back-compat: old name was misleading; keep it as an alias for image tag.
    pub fn with_container_tag(self, image_tag: impl Into<String>) -> Self {
        self.with_image_tag(image_tag)
    }

    pub fn build(self) -> PostgresDependency {
        let postgres_impl = self
            .postgres_impl
            .unwrap_or_else(|| Box::new(PostgresContainerImpl::new()));

        let port = self.port.unwrap_or(Self::DEFAULT_PORT);
        let database_name = self
            .database_name
            .unwrap_or_else(|| Self::DEFAULT_DATABASE_NAME.to_string());
        let database_username = self
            .database_username
            .unwrap_or_else(|| Self::DEFAULT_DATABASE_USERNAME.to_string());
        let database_password = self
            .database_password
            .unwrap_or_else(|| Self::DEFAULT_DATABASE_PASSWORD.to_string());
        let startup_sql_scripts = self.startup_sql_scripts;
        let dependencies = self.dependencies;
        let image_tag = self
            .image_tag
            .unwrap_or_else(|| Self::DEFAULT_IMAGE_TAG.to_string());
        let container_name = self.container_name;
    
        PostgresDependency::new(
            self.identifier,
            postgres_impl,
            port,
            database_name,
            database_username,
            database_password,
            startup_sql_scripts,
            dependencies,
            image_tag,
            container_name,
        )
    }
}