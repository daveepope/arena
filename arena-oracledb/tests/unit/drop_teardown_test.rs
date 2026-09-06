use arena::dependency::RunnableDependency;
use arena::lifecycle::RunnableState;
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
    force_stop_calls: Mutex<u32>,
    release_calls: Mutex<u32>,
}

#[derive(Clone, Default)]
struct RecordingOracleImpl {
    inner: Arc<RecorderInner>,
    fail_on_start: bool,
}

impl RecordingOracleImpl {
    fn stop_call_count(&self) -> u32 {
        *self.inner.stop_calls.lock().expect("stop_calls lock")
    }

    fn force_stop_call_count(&self) -> u32 {
        *self
            .inner
            .force_stop_calls
            .lock()
            .expect("force_stop_calls lock")
    }

    fn release_call_count(&self) -> u32 {
        *self.inner.release_calls.lock().expect("release_calls lock")
    }

    fn failing() -> Self {
        Self {
            inner: Arc::default(),
            fail_on_start: true,
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
    ) -> Result<(), String> {
        if self.fail_on_start {
            return Err("simulated start failure".to_string());
        }
        Ok(())
    }

    async fn stop(&self) -> Result<(), String> {
        *self.inner.stop_calls.lock().expect("stop_calls lock") += 1;
        Ok(())
    }

    async fn force_stop(&self) -> bool {
        *self
            .inner
            .force_stop_calls
            .lock()
            .expect("force_stop_calls lock") += 1;
        true
    }
    fn release(&self) {
        *self.inner.release_calls.lock().expect("release_calls lock") += 1;
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
fn drop_without_start_does_not_tear_down() {
    let recorder = RecordingOracleImpl::default();
    {
        let _dep = OracleDependency::builder("drop-unstarted")
            .with_impl(recorder.clone())
            .with_readiness_check(AlwaysReadyCheck)
            .build();
    }

    assert_eq!(recorder.stop_call_count(), 0);
    assert_eq!(recorder.force_stop_call_count(), 0);
}

#[tokio::test]
async fn drop_after_explicit_stop_does_not_call_stop_again() {
    let recorder = RecordingOracleImpl::default();
    {
        let mut dep = OracleDependency::builder("drop-after-stop")
            .with_impl(recorder.clone())
            .with_readiness_check(AlwaysReadyCheck)
            .build();
        dep.start().await.expect("start should succeed");
        dep.stop().await.expect("stop should succeed");
    }

    assert_eq!(recorder.stop_call_count(), 1);
}

#[tokio::test]
async fn drop_while_running_releases_container() {
    let recorder = RecordingOracleImpl::default();
    {
        let mut dep = OracleDependency::builder("drop-while-running")
            .with_impl(recorder.clone())
            .with_readiness_check(AlwaysReadyCheck)
            .build();
        dep.start().await.expect("start should succeed");
    }

    assert_eq!(recorder.release_call_count(), 1);
}

#[tokio::test]
async fn start_failure_returns_fault_and_forces_stop() {
    let recorder = RecordingOracleImpl::failing();
    let mut dep = OracleDependency::builder("drop-start-fault")
        .with_impl(recorder.clone())
        .with_readiness_check(AlwaysReadyCheck)
        .build();

    let fault = dep.start().await.expect_err("dependency should fault");

    assert_eq!(fault.id, dep.identifier());
    assert!(fault.message.contains("simulated start failure"));
    assert_eq!(dep.state(), RunnableState::Stopped);
    assert_eq!(recorder.force_stop_call_count(), 1);
}

#[tokio::test]
async fn start_failure_then_drop_does_not_force_stop_twice() {
    let recorder = RecordingOracleImpl::failing();
    {
        let mut dep = OracleDependency::builder("drop-start-fault")
            .with_impl(recorder.clone())
            .with_readiness_check(AlwaysReadyCheck)
            .build();
        let _fault = dep.start().await.expect_err("dependency should fault");
    }

    assert_eq!(recorder.force_stop_call_count(), 1);
}

#[tokio::test]
async fn force_stop_called_twice_is_indistinguishable_from_once() {
    let recorder = RecordingOracleImpl::default();
    let mut dep = OracleDependency::builder("force-stop-twice")
        .with_impl(recorder.clone())
        .with_readiness_check(AlwaysReadyCheck)
        .build();

    dep.start().await.expect("start should succeed");
    dep.force_stop().await;
    let after_first = dep.state();
    dep.force_stop().await;

    assert_eq!(after_first, RunnableState::Stopped);
    assert_eq!(dep.state(), RunnableState::Stopped);
    assert!(dep.faults().is_empty());
}
