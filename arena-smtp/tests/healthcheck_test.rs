use arena::dependency::RunnableDependency;
use arena_smtp::{SmtpDependency, SmtpImpl, SmtpTlsConfig};
use async_trait::async_trait;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;

struct FixedAddressSmtpImpl {
    smtp_address: String,
    http_api_url: String,
}

#[async_trait]
impl SmtpImpl for FixedAddressSmtpImpl {
    async fn start(
        &mut self,
        _smtp_port: u16,
        _ui_port: u16,
        _image_name: &str,
        _image_tag: &str,
        _container_name: &str,
        _tls: Option<&SmtpTlsConfig>,
    ) -> Result<(), String> {
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), String> {
        Ok(())
    }
    async fn force_stop(&mut self) -> bool {
        true
    }
    fn release(&mut self) {}


    fn smtp_address(&self) -> Option<&str> {
        Some(&self.smtp_address)
    }

    fn http_api_url(&self) -> Option<&str> {
        Some(&self.http_api_url)
    }
}

#[tokio::test]
async fn default_readiness_check_accepts_real_smtp_banner() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap().to_string();

    tokio::spawn(async move {
        if let Ok((mut socket, _)) = listener.accept().await {
            let _ = socket.write_all(b"220 mailpit ready\r\n").await;
        }
    });

    let mut dep = SmtpDependency::builder("smtp-real-banner")
        .with_impl(FixedAddressSmtpImpl {
            smtp_address: addr,
            http_api_url: "http://127.0.0.1:8025".to_string(),
        })
        .build();

    tokio::time::timeout(Duration::from_secs(5), dep.start())
        .await
        .expect("start should complete against a locally-ready banner")
        .expect("start should succeed");

    dep.stop().await.expect("stop should succeed");
}

#[tokio::test]
async fn default_readiness_check_implicit_tls_ready_without_banner() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap().to_string();

    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((socket, _)) => {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    drop(socket);
                }
                Err(_) => break,
            }
        }
    });

    let mut dep = SmtpDependency::builder("smtp-implicit-tls-ready")
        .with_implicit_tls()
        .with_impl(FixedAddressSmtpImpl {
            smtp_address: addr,
            http_api_url: "http://127.0.0.1:8025".to_string(),
        })
        .build();

    tokio::time::timeout(Duration::from_secs(5), dep.start())
        .await
        .expect("implicit tls readiness should complete on connect without a banner")
        .expect("start should succeed");

    dep.stop().await.expect("stop should succeed");
}

#[tokio::test]
async fn default_readiness_check_retries_after_dropped_connection() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap().to_string();

    tokio::spawn(async move {
        if let Ok((socket, _)) = listener.accept().await {
            drop(socket);
        }
        if let Ok((mut socket, _)) = listener.accept().await {
            let _ = socket.write_all(b"220 mailpit ready\r\n").await;
        }
    });

    let mut dep = SmtpDependency::builder("smtp-real-banner-retry")
        .with_impl(FixedAddressSmtpImpl {
            smtp_address: addr,
            http_api_url: "http://127.0.0.1:8025".to_string(),
        })
        .build();

    tokio::time::timeout(Duration::from_secs(5), dep.start())
        .await
        .expect("start should complete once the second connection succeeds")
        .expect("start should succeed");

    dep.stop().await.expect("stop should succeed");
}
