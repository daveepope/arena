use arena::dependency::RunnableDependency;
use arena::healthcheck::ReadinessCheck;
use arena_oracledb::{OracleDependency, OracleImpl};
use async_trait::async_trait;
use std::sync::{Arc, Mutex};

struct AlwaysReadyCheck;

#[async_trait]
impl ReadinessCheck for AlwaysReadyCheck {
    async fn is_ready(&self, _identifier: &str, _target: &str, _timeout_ms: u64) -> Result<(), String> {
        Ok(())
    }
}

#[derive(Default)]
struct RecorderInner {
    stop_calls: Mutex<u32>,
}

#[derive(Clone, Default)]
struct RecordingOracleImpl {
    inner: Arc<RecorderInner>,
    panic_on_start: bool,
}

impl RecordingOracleImpl {
    fn stop_call_count(&self) -> u32 {
        *self.inner.stop_calls.lock().expect("stop_calls lock")
    }

    fn panicking() -> Self {
        Self {
            inner: Arc::default(),
            panic_on_start: true,
        }
    }
}

#[async_trait]
impl OracleImpl for RecordingOracleImpl {
    #[allow(clippy::too_many_arguments)]
    async fn start(
        &self,
        _port: u16,
        _database_name: &str,
        _database_username: &str,
        _database_password: &str,
        _admin_password: &str,
        _image_name: &str,
        _image_tag: &str,
        _container_name: &str,
    ) {
        if self.panic_on_start {
            panic!("simulated start failure");
        }
    }

    async fn stop(&self) {
        *self.inner.stop_calls.lock().expect("stop_calls lock") += 1;
    }

    fn connection_string(&self) -> Option<String> {
        Some("//localhost:1521/FREEPDB1".to_string())
    }

    fn host_address(&self) -> Option<String> {
        Some("127.0.0.1:1521".to_string())
    }

    async fn run_sqlplus(&self, _username: &str, _password: &str, script: &str) -> Result<String, String> {
        if script.contains("SELECT 1 FROM DUAL") {
            return Ok("1\n".to_string());
        }

        Ok(String::new())
    }
}

#[test]
fn drop_without_start_does_not_call_stop() {
    let recorder = RecordingOracleImpl::default();
    {
        let _dep = OracleDependency::builder("drop-unstarted")
            .with_impl(recorder.clone())
            .with_readiness_check(AlwaysReadyCheck)
            .build();
    }

    assert_eq!(recorder.stop_call_count(), 0);
}

#[tokio::test]
async fn drop_after_explicit_stop_does_not_call_stop_again() {
    let recorder = RecordingOracleImpl::default();
    {
        let mut dep = OracleDependency::builder("drop-after-stop")
            .with_impl(recorder.clone())
            .with_readiness_check(AlwaysReadyCheck)
            .build();
        dep.start().await;
        dep.stop().await;
    }

    assert_eq!(recorder.stop_call_count(), 1);
}

#[tokio::test]
async fn drop_while_running_forces_stop() {
    let recorder = RecordingOracleImpl::default();
    {
        let mut dep = OracleDependency::builder("drop-while-running")
            .with_impl(recorder.clone())
            .with_readiness_check(AlwaysReadyCheck)
            .build();
        dep.start().await;
    }

    assert_eq!(recorder.stop_call_count(), 1);
}

#[test]
fn start_panic_still_triggers_cleanup_on_drop() {
    let recorder = RecordingOracleImpl::panicking();
    let recorder_check = recorder.clone();

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut dep = OracleDependency::builder("drop-start-panic")
            .with_impl(recorder)
            .build();
        futures::executor::block_on(dep.start());
    }));

    assert!(result.is_err());
    assert_eq!(recorder_check.stop_call_count(), 1);
}
