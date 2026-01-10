use crate::postgres_container_impl::PostgresContainerImpl;
use crate::postgres_dependency::{PostgresDependency, PostgresDependencyWrapper};

pub struct PostgresDependencyBuilder {
    name: String,
    wrapper: Option<Box<dyn PostgresDependencyWrapper>>,
    // future:
    // port: Option<u16>,
    // schema_scripts: Vec<String>,
}

impl PostgresDependencyBuilder {
    pub(crate) fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), wrapper: None }
    }

    pub fn with_impl<W>(mut self, wrapper: W) -> Self
    where
        W: PostgresDependencyWrapper + 'static,
    {
        self.wrapper = Some(Box::new(wrapper));
        self
    }

    pub fn build(self) -> PostgresDependency {
        let wrapper = self
            .wrapper
            .unwrap_or_else(|| Box::new(PostgresContainerImpl::new()));

        PostgresDependency::new(self.name, wrapper)
    }
}