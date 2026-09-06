use arena_oracledb::oracle_dependency::oracle_container_impl::{self, OracleImpl};
use async_trait::async_trait;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

fn test_password() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before unix epoch")
        .as_nanos();
    format!("test-pw-{nanos}")
}

struct FakeOracleImpl {
    run_sqlplus_response: Mutex<Option<Result<String, String>>>,
    last_call: Mutex<Option<(String, String, String)>>,
}

impl FakeOracleImpl {
    fn with_response(response: Result<String, String>) -> Self {
        Self {
            run_sqlplus_response: Mutex::new(Some(response)),
            last_call: Mutex::new(None),
        }
    }
}

#[async_trait]
impl OracleImpl for FakeOracleImpl {
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
        Ok(())
    }

    async fn stop(&self) -> Result<(), String> {
        Ok(())
    }
    async fn force_stop(&self) -> bool {
        true
    }
    fn release(&self) {}


    fn connection_string(&self) -> Option<String> {
        None
    }

    fn host_address(&self) -> Option<String> {
        None
    }

    async fn run_sqlplus(&self, username: &str, password: &str, script: &str) -> Result<String, String> {
        *self.last_call.lock().expect("last_call lock") =
            Some((username.to_string(), password.to_string(), script.to_string()));
        self.run_sqlplus_response
            .lock()
            .expect("run_sqlplus_response lock")
            .take()
            .expect("run_sqlplus called more than once on this fake")
    }
}

#[tokio::test]
async fn exec_sql_wraps_sql_in_script_before_calling_run_sqlplus() {
    let fake = FakeOracleImpl::with_response(Ok("OUTPUT".to_string()));
    let password = test_password();

    let result = oracle_container_impl::exec_sql(&fake, "user", &password, "SELECT 1 FROM dual").await;

    assert_eq!(result, Ok("OUTPUT".to_string()));
    let (username, called_password, script) = fake.last_call.lock().expect("last_call lock").clone().unwrap();
    assert_eq!(username, "user");
    assert_eq!(called_password, password);
    assert!(script.contains("SELECT 1 FROM dual"));
    assert!(script.contains("WHENEVER SQLERROR"));
}

#[tokio::test]
async fn exec_sql_run_sqlplus_error_propagates_to_caller() {
    let fake = FakeOracleImpl::with_response(Err("boom".to_string()));

    let result = oracle_container_impl::exec_sql(&fake, "user", &test_password(), "SELECT 1").await;

    assert_eq!(result, Err("boom".to_string()));
}

#[tokio::test]
async fn exec_scalar_query_numeric_stdout_returns_parsed_i32() {
    let fake = FakeOracleImpl::with_response(Ok("  99  \n".to_string()));

    let result =
        oracle_container_impl::exec_scalar_query(&fake, "user", &test_password(), "SELECT COUNT(*) FROM widgets")
            .await;

    assert_eq!(result, Ok(99));
}

#[tokio::test]
async fn exec_scalar_query_non_numeric_stdout_returns_err() {
    let fake = FakeOracleImpl::with_response(Ok("oops".to_string()));

    let result = oracle_container_impl::exec_scalar_query(&fake, "user", &test_password(), "SELECT 1").await;

    assert!(result.is_err());
}

#[tokio::test]
async fn exec_table_list_multiline_stdout_returns_parsed_names() {
    let fake = FakeOracleImpl::with_response(Ok("WIDGETS\nGADGETS\n".to_string()));

    let result =
        oracle_container_impl::exec_table_list(&fake, "user", &test_password(), "SELECT TABLE_NAME FROM USER_TABLES")
            .await;

    assert_eq!(result, Ok(vec!["WIDGETS".to_string(), "GADGETS".to_string()]));
}

#[tokio::test]
async fn exec_constraint_list_pipe_delimited_stdout_returns_parsed_pairs() {
    let fake = FakeOracleImpl::with_response(Ok("WIDGETS|FK_1\n".to_string()));

    let result = oracle_container_impl::exec_constraint_list(
        &fake,
        "user",
        &test_password(),
        "SELECT table_name || '|' || constraint_name FROM user_constraints",
    )
    .await;

    assert_eq!(result, Ok(vec![("WIDGETS".to_string(), "FK_1".to_string())]));
}
