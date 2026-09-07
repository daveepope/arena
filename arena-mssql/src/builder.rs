use crate::mssql_dependency::mssql_container_impl::{
    MssqlContainerImpl, MssqlEncryption, MssqlImpl,
};
use crate::mssql_dependency::MssqlDependency;
use arena::dependency::RunnableDependency;
use arena::healthcheck::ReadinessCheck;
use std::time::Duration;

pub struct MssqlDependencyBuilder {
    identifier: String,
    expiry: Option<Duration>,
    mssql_impl: Option<Box<dyn MssqlImpl>>,
    port: Option<u16>,
    database_name: Option<String>,
    database_username: Option<String>,
    database_password: Option<String>,
    startup_sql_scripts: Option<Vec<String>>,
    dependencies: Option<Vec<Box<dyn RunnableDependency>>>,
    image_name: Option<String>,
    image_tag: Option<String>,
    container_name: Option<String>,
    network: Option<String>,
    readiness_check: Option<Box<dyn ReadinessCheck>>,
    connect_timeout: Option<Option<Duration>>,
    encryption: Option<MssqlEncryption>,
}

impl MssqlDependencyBuilder {
    const DEFAULT_PORT: u16 = 1433;
    const DEFAULT_DATABASE_NAME: &'static str = "arena_db";
    const DEFAULT_DATABASE_USERNAME: &'static str = "sa";
    const DEFAULT_DATABASE_PASSWORD: &'static str = "yourStrong(!)Password";
    const DEFAULT_IMAGE_NAME: &'static str = arena_container::default_images::MSSQL.image;
    const DEFAULT_IMAGE_TAG: &'static str = arena_container::default_images::MSSQL.tag;

    pub(crate) fn new(identifier: impl Into<String>) -> Self {
        Self {
            identifier: identifier.into(),
            expiry: Some(arena_container::expiry::DEFAULT_EXPIRY),
            mssql_impl: None,
            port: None,
            database_name: None,
            database_username: None,
            database_password: None,
            startup_sql_scripts: None,
            dependencies: None,
            image_name: None,
            image_tag: None,
            container_name: None,
            network: None,
            readiness_check: None,
            connect_timeout: None,
            encryption: None,
        }
    }

    pub fn with_impl<W>(mut self, wrapper: W) -> Self
    where
        W: MssqlImpl + 'static,
    {
        self.mssql_impl = Some(Box::new(wrapper));
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

    pub fn with_connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = Some(Some(timeout));
        self
    }

    pub fn without_connect_timeout(mut self) -> Self {
        self.connect_timeout = Some(None);
        self
    }

    pub fn with_encryption(mut self, encryption: MssqlEncryption) -> Self {
        self.encryption = Some(encryption);
        self
    }

    pub fn with_container_tag(self, image_tag: impl Into<String>) -> Self {
        self.with_image_tag(image_tag)
    }

    pub fn with_expiry(mut self, expiry: Duration) -> Self {
        self.expiry = Some(expiry);
        self
    }

    pub fn without_expiry(mut self) -> Self {
        self.expiry = None;
        self
    }

    pub fn build(self) -> MssqlDependency {
        let encryption = self.encryption.unwrap_or_default();
        let mut mssql_impl = self
            .mssql_impl
            .unwrap_or_else(|| Box::new(MssqlContainerImpl::new(self.network, encryption)));
        mssql_impl.set_expiry(self.expiry);

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
        let image_name = self
            .image_name
            .unwrap_or_else(|| Self::DEFAULT_IMAGE_NAME.to_string());
        let image_tag = self
            .image_tag
            .unwrap_or_else(|| Self::DEFAULT_IMAGE_TAG.to_string());
        let container_name = self.container_name;
        let readiness_check = self.readiness_check;

        let mut dep = MssqlDependency::new(
            arena_container::identifier::build(crate::MODULE, &self.identifier),
            mssql_impl,
            port,
            database_name,
            database_username,
            database_password,
            startup_sql_scripts,
            dependencies,
            image_name,
            image_tag,
            container_name,
        );

        if let Some(check) = readiness_check {
            dep.set_readiness_check(check);
        }

        if let Some(timeout) = self.connect_timeout {
            dep.set_connect_timeout(timeout);
        }

        dep
    }
}
