use arena::dependency::RunnableDependency;
use async_trait::async_trait;
use crate::builder::KafkaDependencyBuilder;
use crate::kafka_container_impl::KafkaImpl;
use futures_timer::Delay;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

trait ProbeTransport {
    type Stream: Read + Write;
    fn connect(&self, bootstrap: &str) -> Result<Self::Stream, String>;
}

struct TcpProbeTransport;

impl ProbeTransport for TcpProbeTransport {
    type Stream = TcpStream;

    fn connect(&self, bootstrap: &str) -> Result<Self::Stream, String> {
        let addr = KafkaDependency::resolve_bootstrap_addr(bootstrap)?;
        KafkaDependency::connect_with_timeout(addr)
    }
}

pub struct KafkaDependency {
    pub identifier: String,
    kafka_impl: Box<dyn KafkaImpl>,
    port: u16,
    dependencies: Option<Vec<Box<dyn RunnableDependency>>>,
    running: bool,
    container_tag: String,
}

impl KafkaDependency {
    pub fn new(
        identifier: String,
        kafka_impl: Box<dyn KafkaImpl>,
        port: u16,
        dependencies: Option<Vec<Box<dyn RunnableDependency>>>,
        container_tag: String,
    ) -> Self {
        KafkaDependency {
            identifier,
            kafka_impl,
            port,
            dependencies,
            container_tag,
            running: false,
        }
    }

    pub fn bootstrap_servers(&self) -> Option<&str> {
        self.kafka_impl.bootstrap_servers()
    }

    pub fn builder(identifier: impl Into<String>) -> KafkaDependencyBuilder {
        KafkaDependencyBuilder::new(identifier)
    }

    fn readiness_bootstrap_on_host(&self) -> Result<&str, String> {
        self.bootstrap_servers()
            .ok_or_else(|| "kafka bootstrap servers not available yet".to_string())
    }

    fn resolve_bootstrap_addr(bootstrap: &str) -> Result<SocketAddr, String> {
        let addr: SocketAddr = bootstrap
            .to_socket_addrs()
            .map_err(|e| format!("resolve bootstrap {bootstrap:?} failed: {e}"))?
            .next()
            .ok_or_else(|| format!("resolve bootstrap {bootstrap:?} produced no addresses"))?;
        Ok(addr)
    }

    fn connect_with_timeout(addr: SocketAddr) -> Result<TcpStream, String> {
        let stream = TcpStream::connect_timeout(&addr, Duration::from_millis(250))
            .map_err(|e| format!("tcp connect to {addr} failed: {e}"))?;
        stream
            .set_read_timeout(Some(Duration::from_millis(250)))
            .ok();
        stream
            .set_write_timeout(Some(Duration::from_millis(250)))
            .ok();
        Ok(stream)
    }

    fn build_api_versions_request_v0(correlation_id: i32) -> Result<Vec<u8>, String> {
        const API_KEY_API_VERSIONS: i16 = 18;
        const API_VERSION: i16 = 0;

        let mut body: Vec<u8> = Vec::with_capacity(16);
        body.extend_from_slice(&API_KEY_API_VERSIONS.to_be_bytes());
        body.extend_from_slice(&API_VERSION.to_be_bytes());
        body.extend_from_slice(&correlation_id.to_be_bytes());
        body.extend_from_slice(&(0i16).to_be_bytes()); // client_id length = 0

        let len: i32 = body
            .len()
            .try_into()
            .map_err(|_| "request too large".to_string())?;

        let mut frame: Vec<u8> = Vec::with_capacity(4 + body.len());
        frame.extend_from_slice(&len.to_be_bytes());
        frame.extend_from_slice(&body);
        Ok(frame)
    }

    fn write_kafka_frame<S: Write>(stream: &mut S, frame: &[u8]) -> Result<(), String> {
        stream
            .write_all(frame)
            .map_err(|e| format!("kafka probe write failed: {e}"))?;
        stream.flush().ok();
        Ok(())
    }

    fn read_kafka_frame<S: Read>(stream: &mut S) -> Result<Vec<u8>, String> {
        let mut size_buf = [0u8; 4];
        stream
            .read_exact(&mut size_buf)
            .map_err(|e| format!("kafka probe read size failed: {e}"))?;
        let size = i32::from_be_bytes(size_buf);
        if size <= 0 || size > 1024 * 1024 {
            return Err(format!("kafka probe invalid response size: {size}"));
        }

        let mut resp = vec![0u8; size as usize];
        stream
            .read_exact(&mut resp)
            .map_err(|e| format!("kafka probe read payload failed: {e}"))?;
        Ok(resp)
    }

    fn parse_response_correlation_id(resp: &[u8]) -> Result<i32, String> {
        if resp.len() < 4 {
            return Err("kafka probe response too short".to_string());
        }
        Ok(i32::from_be_bytes([resp[0], resp[1], resp[2], resp[3]]))
    }

    fn kafka_probe_api_versions_with_transport<T: ProbeTransport>(
        transport: &T,
        bootstrap: &str,
    ) -> Result<(), String> {
        const CORRELATION_ID: i32 = 1;
        let mut stream = transport.connect(bootstrap)?;
        let frame = Self::build_api_versions_request_v0(CORRELATION_ID)?;
        Self::write_kafka_frame(&mut stream, &frame)?;
        let resp = Self::read_kafka_frame(&mut stream)?;
        let corr = Self::parse_response_correlation_id(&resp)?;
        if corr != CORRELATION_ID {
            return Err(format!(
                "kafka probe correlation mismatch: expected {CORRELATION_ID} got {corr}"
            ));
        }
        Ok(())
    }

    fn kafka_probe_api_versions(bootstrap: &str) -> Result<(), String> {
        Self::kafka_probe_api_versions_with_transport(&TcpProbeTransport, bootstrap)
    }

    async fn wait_for_protocol_ready(&self) {
        let timeout = Duration::from_secs(15);
        let poll_every = Duration::from_millis(250);
        let start = Instant::now();

        loop {
            if start.elapsed() >= timeout {
                panic!(
                    "[Kafka-{}] kafka did not become ready within {:?}",
                    self.identifier, timeout
                );
            }

            let bootstrap = match self.readiness_bootstrap_on_host() {
                Ok(v) => v.to_string(),
                Err(err) => {
                    log::debug!("[Kafka-{}] readiness bootstrap missing: {}", self.identifier, err);
                    Delay::new(poll_every).await;
                    continue;
                }
            };

            match Self::kafka_probe_api_versions(&bootstrap) {
                Ok(()) => return,
                Err(err) => {
                    log::debug!(
                        "[Kafka-{}] readiness check failed (will retry): {}",
                        self.identifier,
                        err
                    );
                    Delay::new(poll_every).await;
                }
            }
        }
    }

    async fn is_ready(&self) {
        self.wait_for_protocol_ready().await;
    }
}

#[async_trait]
impl RunnableDependency for KafkaDependency {
    async fn start(&mut self) {
        if self.running {
            return;
        }

        log::info!("[Kafka-{}] starting.", self.identifier);
        let sw = Instant::now();

        for dep in self.dependencies.iter_mut().flatten() {
            dep.start().await;
        }

        let container_tag = self.container_tag.clone();

        let sw_container = Instant::now();
        self.kafka_impl.start(self.port, &container_tag).await;
        log::debug!(
            "[Kafka-{}] container start in {:?}.",
            self.identifier,
            sw_container.elapsed()
        );

        let sw_ready = Instant::now();
        self.is_ready().await;
        log::debug!(
            "[Kafka-{}] readiness in {:?}.",
            self.identifier,
            sw_ready.elapsed()
        );

        self.running = true;
        log::debug!(
            "[Kafka-{}] start complete in {:?}.",
            self.identifier,
            sw.elapsed()
        );
        log::info!("[Kafka-{}] started.", self.identifier);
    }

    async fn stop(&mut self) {
        if !self.running {
            return;
        }

        log::info!("[Kafka-{}] stopping.", self.identifier);
        let sw = Instant::now();

        self.kafka_impl.stop().await;

        for dep in self.dependencies.iter_mut().flatten().rev() {
            dep.stop().await;
        }

        self.running = false;
        log::debug!(
            "[Kafka-{}] stop complete in {:?}.",
            self.identifier,
            sw.elapsed()
        );
        log::info!("[Kafka-{}] stopped.", self.identifier);
    }

    fn add_child(&mut self, dep: Box<dyn RunnableDependency>) {
        self.dependencies.get_or_insert_with(Vec::new).push(dep);
    }
}

#[cfg(test)]
mod tests {
    use super::KafkaDependency;
    use super::ProbeTransport;
    use std::io::{Cursor, Read, Write};

    struct FakeStream {
        read: Cursor<Vec<u8>>,
        written: Vec<u8>,
    }

    impl Read for FakeStream {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.read.read(buf)
        }
    }

    impl Write for FakeStream {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.written.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    struct FakeTransport {
        stream: FakeStream,
    }

    impl ProbeTransport for FakeTransport {
        type Stream = FakeStream;

        fn connect(&self, _bootstrap: &str) -> Result<Self::Stream, String> {
            Ok(FakeStream {
                read: Cursor::new(self.stream.read.get_ref().clone()),
                written: Vec::new(),
            })
        }
    }

    fn response_frame_with_corr(corr: i32) -> Vec<u8> {
        let payload = corr.to_be_bytes().to_vec();
        let mut out = Vec::new();
        out.extend_from_slice(&(payload.len() as i32).to_be_bytes());
        out.extend_from_slice(&payload);
        out
    }

    #[test]
    fn probe_api_versions_succeeds_with_matching_correlation_id() {
        let transport = FakeTransport {
            stream: FakeStream {
                read: Cursor::new(response_frame_with_corr(1)),
                written: Vec::new(),
            },
        };

        KafkaDependency::kafka_probe_api_versions_with_transport(&transport, "ignored")
            .expect("probe should succeed");
    }

    #[test]
    fn probe_api_versions_fails_with_mismatched_correlation_id() {
        let transport = FakeTransport {
            stream: FakeStream {
                read: Cursor::new(response_frame_with_corr(2)),
                written: Vec::new(),
            },
        };

        let err = KafkaDependency::kafka_probe_api_versions_with_transport(&transport, "ignored")
            .expect_err("probe should fail");
        assert!(err.contains("correlation mismatch"), "err was: {err}");
    }
}