mod builder;
mod discovery;
pub use arena_cryptography::ephemeral_tls::localhost_self_signed_pem_pair;
mod loopback_tls_json;
pub use crate::loopback_tls_json::loopback_tls_pem_json_document;
mod keys;
mod oauth_common;
mod oauth_dependency;
mod oauth_ffi;
mod oauth_https;
mod oauth_server;
mod provider;
mod token;

pub use crate::builder::{IssuerConfig, OauthDependencyBuilder};
pub use crate::oauth_dependency::OauthDependency;
pub use crate::provider::Provider;
pub use crate::oauth_ffi::{
    build_oauth_dependency_from_config, OauthFfiDependencyConfig, OauthFfiInboundTransport,
    OauthFfiIssuerConfig,
};
pub use crate::token::{ensure_scopes, validate_scopes, AccessTokenClaims, TokenError};
