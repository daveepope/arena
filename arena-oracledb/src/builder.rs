use crate::oracle_dependency::oracle_container_impl::{
    OracleContainerImpl, OracleImpl, DEFAULT_SERVICE_NAME,
};
use crate::oracle_dependency::OracleDependency;
use arena::dependency::RunnableDependency;
use arena::healthcheck::ReadinessCheck;
use std::sync::Arc;

pub struct OracleDependencyBuilder {
    identifier: String,
    oracle_impl: Option<Arc<dyn OracleImpl>>,
    port: Option<u16>,
    database_name: Option<String>,
    database_username: Option<String>,
    database_password: Option<String>,
    admin_password: Option<String>,
    startup_sql_scripts: Option<Vec<String>>,
    dependencies: Option<Vec<Box<dyn RunnableDependency>>>,
    image_name: Option<String>,
    image_tag: Option<String>,
    container_name: Option<String>,
    network: Option<String>,
    readiness_check: Option<Box<dyn ReadinessCheck>>,
    sql_readiness_timeout: Option<std::time::Duration>,
}

impl OracleDependencyBuilder {
    const DEFAULT_PORT: u16 = 1521;
    const DEFAULT_DATABASE_USERNAME: &'static str = "arena_user";
    const DEFAULT_DATABASE_PASSWORD: &'static str = "ArenaOracle1!";
    const DEFAULT_ADMIN_PASSWORD: &'static str = "ArenaOracleAdmin1!";
    const DEFAULT_IMAGE_NAME: &'static str = arena_container::default_images::ORACLE.image;
    const DEFAULT_IMAGE_TAG: &'static str = arena_container::default_images::ORACLE.tag;

    pub(crate) fn new(identifier: impl Into<String>) -> Self {
        Self {
            identifier: identifier.into(),
            oracle_impl: None,
            port: None,
            database_name: None,
            database_username: None,
            database_password: None,
            admin_password: None,
            startup_sql_scripts: None,
            dependencies: None,
            image_name: None,
            image_tag: None,
            container_name: None,
            network: None,
            readiness_check: None,
            sql_readiness_timeout: None,
        }
    }

    pub fn with_impl<W>(mut self, wrapper: W) -> Self
    where
        W: OracleImpl + 'static,
    {
        self.oracle_impl = Some(Arc::new(wrapper));
        self
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.port = Option::from(port);
        self
    }

    pub fn with_database_name(mut self, database_name: impl Into<String>) -> Self {
        self.database_name = Option::from(database_name.into());
        self
    }

    pub fn with_database_username(mut self, database_username: impl Into<String>) -> Self {
        self.database_username = Option::from(database_username.into());
        self
    }

    pub fn with_database_password(mut self, database_password: impl Into<String>) -> Self {
        self.database_password = Option::from(database_password.into());
        self
    }

    pub fn with_admin_password(mut self, admin_password: impl Into<String>) -> Self {
        self.admin_password = Option::from(admin_password.into());
        self
    }

    pub fn with_startup_sql_scripts(mut self, scripts: Vec<String>) -> Self {
        self.startup_sql_scripts = Option::from(scripts);
        self
    }

    pub fn with_child_dependencies(
        mut self,
        dependencies: Vec<Box<dyn RunnableDependency>>,
    ) -> Self {
        self.dependencies = Option::from(dependencies);
        self
    }

    pub fn with_image_name(mut self, image_name: impl Into<String>) -> Self {
        self.image_name = Some(image_name.into());
        self
    }

    pub fn with_image_tag(mut self, image_tag: impl Into<String>) -> Self {
        self.image_tag = Some(image_tag.into());
        self
    }

    pub fn with_image(self, image_tag: impl Into<String>) -> Self {
        self.with_image_tag(image_tag)
    }

    pub fn with_container_name(mut self, container_name: impl Into<String>) -> Self {
        self.container_name = Some(container_name.into());
        self
    }

    pub fn with_network(mut self, network: impl Into<String>) -> Self {
        self.network = Some(network.into());
        self
    }

    pub fn with_readiness_check<W>(mut self, check: W) -> Self
    where
        W: ReadinessCheck + 'static,
    {
        self.readiness_check = Some(Box::new(check));
        self
    }

    pub fn with_sql_readiness_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.sql_readiness_timeout = Some(timeout);
        self
    }

    pub fn with_container_tag(self, image_tag: impl Into<String>) -> Self {
        self.with_image_tag(image_tag)
    }

    pub fn build(self) -> OracleDependency {
        let oracle_impl = self
            .oracle_impl
            .unwrap_or_else(|| Arc::new(OracleContainerImpl::new(self.network)));

        let port = self.port.unwrap_or(Self::DEFAULT_PORT);
        let database_name = self
            .database_name
            .unwrap_or_else(|| DEFAULT_SERVICE_NAME.to_string());
        let database_username = self
            .database_username
            .unwrap_or_else(|| Self::DEFAULT_DATABASE_USERNAME.to_string());
        let database_password = self
            .database_password
            .unwrap_or_else(|| Self::DEFAULT_DATABASE_PASSWORD.to_string());
        let admin_password = self
            .admin_password
            .unwrap_or_else(|| Self::DEFAULT_ADMIN_PASSWORD.to_string());
        let startup_sql_scripts = self.startup_sql_scripts;
        let dependencies = self.dependencies;
        let image_name = self
            .image_name
            .unwrap_or_else(|| Self::DEFAULT_IMAGE_NAME.to_string());
        let image_tag = self
            .image_tag
            .unwrap_or_else(|| Self::DEFAULT_IMAGE_TAG.to_string());
        let container_name = self.container_name;
        let readiness_check = self.readiness_check;

        let mut dep = OracleDependency::new(
            arena_container::identifier::build("arena-oracledb", &self.identifier),
            oracle_impl,
            port,
            database_name,
            database_username,
            database_password,
            admin_password,
            startup_sql_scripts,
            dependencies,
            image_name,
            image_tag,
            container_name,
        );

        if let Some(check) = readiness_check {
            dep.set_readiness_check(check);
        }
        if let Some(timeout) = self.sql_readiness_timeout {
            dep.set_sql_readiness_timeout(timeout);
        }

        dep
    }
}
