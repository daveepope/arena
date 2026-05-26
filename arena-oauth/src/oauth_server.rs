use std::net::{IpAddr, SocketAddr, TcpListener};
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum_server::tls_rustls::RustlsConfig;
use axum_server::{bind, bind_rustls, Handle};
use std::sync::Once;

use crate::discovery::OAuthAuthorizationServerMetadata;
use crate::keys::RsaKeyPair;
use crate::oauth_common::{OAuthSigningState, OauthListenAddr};
use crate::oauth_https::https_router;

static RUSTLS_CRYPTO_INSTALL: Once = Once::new();

fn ensure_rustls_default_crypto_provider() {
    RUSTLS_CRYPTO_INSTALL.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

struct OauthServerStarted {
    handle: Handle<SocketAddr>,
    join: tokio::task::JoinHandle<std::result::Result<(), std::io::Error>>,
    base_url: String,
    readiness_poll_base: String,
    signing_state: Arc<OAuthSigningState>,
}

#[derive(Default)]
pub(crate) struct OauthServer {
    inner: Option<OauthServerStarted>,
}

fn reserve_bind_port(listen_ip: IpAddr) -> u16 {
    let l = TcpListener::bind(SocketAddr::new(listen_ip, 0)).expect("bind ephemeral port for TLS");
    let p = l.local_addr().expect("local_addr").port();
    drop(l);
    p
}

fn origin_base_url(scheme: &str, listen_ip: IpAddr, bind_port: u16) -> String {
    match listen_ip {
        IpAddr::V4(v4) => format!("{scheme}://{v4}:{bind_port}"),
        IpAddr::V6(v6) => format!("{scheme}://[{v6}]:{bind_port}"),
    }
}

impl OauthServer {
    pub(crate) async fn start(
        &mut self,
        log_label: &str,
        listen: OauthListenAddr,
        keys: RsaKeyPair,
        scopes_supported: Vec<String>,
        token_ttl_secs: u64,
        tls_pem: Option<(String, String)>,
        metadata_base_url_override: Option<String>,
    ) {
        assert!(
            self.inner.is_none(),
            "[Oauth-{log_label}] oauth server already running"
        );

        let scheme = if tls_pem.is_some() { "https" } else { "http" };

        let bind_port = if listen.port == 0 {
            reserve_bind_port(listen.ip)
        } else {
            listen.port
        };
        let readiness_poll_base = if listen.ip.is_unspecified() {
            format!("{scheme}://127.0.0.1:{bind_port}")
        } else {
            origin_base_url(scheme, listen.ip, bind_port)
        };
        let metadata_base = metadata_base_url_override
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.trim_end_matches('/').to_string())
            .unwrap_or_else(|| readiness_poll_base.trim_end_matches('/').to_string());

        let metadata = Arc::new(OAuthAuthorizationServerMetadata::for_base_url(
            &metadata_base,
            scopes_supported,
        ));
        let signing_state = Arc::new(OAuthSigningState {
            metadata,
            keys: Arc::new(keys),
            token_ttl_secs,
        });

        let addr = SocketAddr::new(listen.ip, bind_port);
        let router = https_router(signing_state.clone());
        let handle = Handle::new();

        let join = match tls_pem {
            Some((cert_pem, key_pem)) => {
                ensure_rustls_default_crypto_provider();
                let rustls = RustlsConfig::from_pem(cert_pem.into_bytes(), key_pem.into_bytes())
                    .await
                    .unwrap_or_else(|e| panic!("[Oauth-{log_label}] invalid TLS PEM: {e}"));
                let server = bind_rustls(addr, rustls).handle(handle.clone());
                tokio::spawn(async move { server.serve(router.into_make_service()).await })
            }
            None => {
                let server = bind(addr).handle(handle.clone());
                tokio::spawn(async move { server.serve(router.into_make_service()).await })
            }
        };

        self.inner = Some(OauthServerStarted {
            handle,
            join,
            base_url: metadata_base,
            readiness_poll_base,
            signing_state,
        });
    }

    pub(crate) async fn stop(&mut self) {
        if let Some(inner) = self.inner.take() {
            inner.handle.graceful_shutdown(None);
            let _ = inner.join.await;
        }
    }

    pub(crate) fn base_url(&self) -> Option<&str> {
        self.inner.as_ref().map(|s| s.base_url.as_str())
    }

    pub(crate) fn signing_state(&self) -> Option<&Arc<OAuthSigningState>> {
        self.inner.as_ref().map(|s| &s.signing_state)
    }

    pub(crate) async fn wait_until_ready(&self, log_label: &str) {
        let timeout = Duration::from_secs(15);
        let poll_every = Duration::from_millis(100);
        let start = Instant::now();
        let poll_base = self
            .inner
            .as_ref()
            .map(|s| s.readiness_poll_base.as_str())
            .unwrap_or_else(|| panic!("[Oauth-{log_label}] readiness: server not started"));
        let url = format!("{poll_base}/.well-known/oauth-authorization-server");
        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .expect("reqwest client");

        tracing::debug!(
            subsystem = "oauth",
            log_label = log_label,
            url = %url,
            overall = ?timeout,
            poll_every = ?poll_every,
            "readiness probe loop starting"
        );

        let mut attempt: u64 = 0;
        let mut last_outcome: Option<String> = None;

        loop {
            if start.elapsed() >= timeout {
                panic!(
                    "[Oauth-{log_label}] did not become ready within {:?}. url={url}, attempts={attempt}, last_outcome={:?}",
                    timeout, last_outcome
                );
            }

            attempt = attempt.saturating_add(1);
            let attempt_started = Instant::now();

            match client.get(&url).send().await {
                Ok(r) if r.status().is_success() => {
                    tracing::debug!(
                        subsystem = "oauth",
                        log_label = log_label,
                        attempts = attempt,
                        elapsed_total = ?start.elapsed(),
                        status = %r.status(),
                        "readiness probe succeeded"
                    );
                    break;
                }
                Ok(r) => {
                    let status = r.status();
                    last_outcome = Some(format!("http {status}"));
                    tracing::error!(
                        subsystem = "oauth",
                        log_label = log_label,
                        attempt = attempt,
                        elapsed = ?attempt_started.elapsed(),
                        status = %status,
                        "readiness probe non-success (will retry)"
                    );
                }
                Err(e) => {
                    last_outcome = Some(format!("send error: {e}"));
                    tracing::error!(
                        subsystem = "oauth",
                        log_label = log_label,
                        attempt = attempt,
                        elapsed = ?attempt_started.elapsed(),
                        error = %e,
                        "readiness probe send failed (will retry)"
                    );
                }
            }
            tokio::time::sleep(poll_every).await;
        }
    }
}
