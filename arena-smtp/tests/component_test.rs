use arena::dependency::RunnableDependency;
use arena_smtp::SmtpDependency;
use futures::FutureExt;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

fn init_test_logging() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_test_writer()
        .try_init();
}

struct TestContext {
    smtp: SmtpDependency,
    smtp_address: String,
    http_api_url: String,
}

impl TestContext {
    async fn new() -> Result<Self, String> {
        let mut smtp = SmtpDependency::builder("")
            .with_port(12025)
            .with_ui_port(18025)
            .build();

        let start_outcome = std::panic::AssertUnwindSafe(async { smtp.start().await })
            .catch_unwind()
            .await;
        if let Err(panic_payload) = start_outcome {
            smtp.stop().await;
            std::panic::resume_unwind(panic_payload);
        }

        let smtp_address = smtp
            .smtp_address()
            .ok_or_else(|| "smtp address missing after start()".to_string())?
            .to_string();
        let http_api_url = smtp
            .http_api_url()
            .ok_or_else(|| "smtp http api url missing after start()".to_string())?
            .to_string();

        Ok(Self {
            smtp,
            smtp_address,
            http_api_url,
        })
    }

    async fn stop(mut self) {
        self.smtp.stop().await;
    }
}

async fn read_reply(stream: &mut TcpStream, expected_prefix: &str) -> Result<(), String> {
    let mut buffer = [0u8; 512];
    let read = tokio::time::timeout(Duration::from_secs(5), stream.read(&mut buffer))
        .await
        .map_err(|_| "smtp reply timed out".to_string())?
        .map_err(|err| err.to_string())?;
    let reply = String::from_utf8_lossy(&buffer[..read]);
    if reply.starts_with(expected_prefix) {
        Ok(())
    } else {
        Err(format!("expected reply prefix {expected_prefix}, got {reply}"))
    }
}

async fn send_line(stream: &mut TcpStream, line: &str) -> Result<(), String> {
    stream
        .write_all(line.as_bytes())
        .await
        .map_err(|err| err.to_string())
}

async fn send_probe_message(smtp_address: &str, subject: &str) -> Result<(), String> {
    let mut stream = TcpStream::connect(smtp_address)
        .await
        .map_err(|err| err.to_string())?;

    read_reply(&mut stream, "220").await?;
    send_line(&mut stream, "EHLO arena\r\n").await?;
    read_reply(&mut stream, "250").await?;
    send_line(&mut stream, "MAIL FROM:<sender@arena.test>\r\n").await?;
    read_reply(&mut stream, "250").await?;
    send_line(&mut stream, "RCPT TO:<recipient@arena.test>\r\n").await?;
    read_reply(&mut stream, "250").await?;
    send_line(&mut stream, "DATA\r\n").await?;
    read_reply(&mut stream, "354").await?;
    let body = format!(
        "Subject: {subject}\r\nFrom: sender@arena.test\r\nTo: recipient@arena.test\r\n\r\nhello from arena\r\n.\r\n"
    );
    send_line(&mut stream, &body).await?;
    read_reply(&mut stream, "250").await?;
    send_line(&mut stream, "QUIT\r\n").await?;
    read_reply(&mut stream, "221").await?;
    Ok(())
}

async fn ehlo_extensions(smtp_address: &str) -> Result<String, String> {
    let mut stream = TcpStream::connect(smtp_address)
        .await
        .map_err(|err| err.to_string())?;
    read_reply(&mut stream, "220").await?;
    send_line(&mut stream, "EHLO arena\r\n").await?;

    let mut buffer = [0u8; 1024];
    let read = tokio::time::timeout(Duration::from_secs(5), stream.read(&mut buffer))
        .await
        .map_err(|_| "ehlo read timed out".to_string())?
        .map_err(|err| err.to_string())?;
    let response = String::from_utf8_lossy(&buffer[..read]).to_string();
    let _ = send_line(&mut stream, "QUIT\r\n").await;
    Ok(response)
}

async fn captured_message_count(http_api_url: &str, subject: &str) -> Result<usize, String> {
    let url = format!("{http_api_url}/api/v1/messages");
    let body = reqwest::get(&url)
        .await
        .map_err(|err| err.to_string())?
        .text()
        .await
        .map_err(|err| err.to_string())?;
    Ok(body.matches(subject).count())
}

#[tokio::test]
async fn smtp_dependency_captures_sent_message_component_test() {
    init_test_logging();

    let ctx = match TestContext::new().await {
        Ok(v) => v,
        Err(e) => panic!("{e}"),
    };

    let subject = format!(
        "arena-smtp-probe-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );

    let outcome = std::panic::AssertUnwindSafe(async {
        assert!(
            !ctx.http_api_url.is_empty(),
            "expected a non-empty smtp http api url"
        );

        send_probe_message(&ctx.smtp_address, &subject).await?;

        let mut captured = 0;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
        while tokio::time::Instant::now() < deadline {
            captured = captured_message_count(&ctx.http_api_url, &subject).await?;
            if captured >= 1 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        if captured >= 1 {
            Ok(())
        } else {
            Err(format!("sent message {subject} was not captured"))
        }
    })
    .catch_unwind()
    .await;

    tokio::time::timeout(Duration::from_secs(10), ctx.stop())
        .await
        .unwrap_or_else(|_| panic!("smtp stop timed out"));

    match outcome {
        Ok(Ok(())) => {}
        Ok(Err(e)) => panic!("{e}"),
        Err(panic_payload) => std::panic::resume_unwind(panic_payload),
    }
}

#[tokio::test]
async fn smtp_dependency_with_starttls_advertises_starttls_component_test() {
    init_test_logging();

    let mut smtp = SmtpDependency::builder("")
        .with_starttls()
        .with_port(12026)
        .with_ui_port(18026)
        .build();

    let start_outcome = std::panic::AssertUnwindSafe(async { smtp.start().await })
        .catch_unwind()
        .await;
    if let Err(panic_payload) = start_outcome {
        smtp.stop().await;
        std::panic::resume_unwind(panic_payload);
    }

    let smtp_address = smtp
        .smtp_address()
        .expect("smtp address missing after start()")
        .to_string();

    let outcome = std::panic::AssertUnwindSafe(async {
        let extensions = ehlo_extensions(&smtp_address).await?;
        if extensions.contains("STARTTLS") {
            Ok(())
        } else {
            Err(format!("EHLO did not advertise STARTTLS: {extensions}"))
        }
    })
    .catch_unwind()
    .await;

    tokio::time::timeout(Duration::from_secs(10), smtp.stop())
        .await
        .unwrap_or_else(|_| panic!("smtp stop timed out"));

    match outcome {
        Ok(Ok(())) => {}
        Ok(Err(e)) => panic!("{e}"),
        Err(panic_payload) => std::panic::resume_unwind(panic_payload),
    }
}
