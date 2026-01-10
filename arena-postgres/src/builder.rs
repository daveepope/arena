use arena::dependency::RunnableDependency;
use crate::postgres_container_impl::{PostgresContainerImpl, PostgresImpl};
use crate::postgres_dependency::{PostgresDependency};

pub struct PostgresDependencyBuilder {
    name: String,
    postgres_impl: Option<Box<dyn PostgresImpl>>,
    port: Option<u16>,
    startup_sql_scripts: Option<Vec<String>>,
    dependencies: Option<Vec<Box<dyn RunnableDependency>>>
}

impl PostgresDependencyBuilder {

    const DEFAULT_PORT: u16 = 5432;

    pub(crate) fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), postgres_impl: None , port: None, startup_sql_scripts: None , dependencies: None}
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

    pub fn build(self) -> PostgresDependency {
        let postgres_impl = self
            .postgres_impl
            .unwrap_or_else(|| Box::new(PostgresContainerImpl::new()));

        let port = self.port.unwrap_or(Self::DEFAULT_PORT);
        let startup_sql_scripts = self.startup_sql_scripts;
        let dependencies = self.dependencies;
    
        PostgresDependency::new(self.name, postgres_impl, port, startup_sql_scripts, dependencies)
    }
}