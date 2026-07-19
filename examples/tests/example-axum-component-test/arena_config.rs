use serde_json::Value;
use std::path::Path;
use std::sync::OnceLock;

struct ArenaConfig {
    calibration_validate_path: String,
    calibration_api_happy_path_id: String,
    calibration_api_error_path_id: String,
    calibration_api_flaky_path_id: String,
    reset_validation_db_id: String,
    #[allow(dead_code)]
    events_purge_id: String,
}

static CONFIG: OnceLock<ArenaConfig> = OnceLock::new();

fn find_config_path() -> Option<String> {
    if let Ok(runfiles) = std::env::var("RUNFILES_DIR") {
        for base in ["_main", "arena", ""] {
            let path = if base.is_empty() {
                Path::new(&runfiles).join("examples/resources/arena_config.json")
            } else {
                Path::new(&runfiles)
                    .join(base)
                    .join("examples/resources/arena_config.json")
            };
            if path.is_file() {
                return Some(path.to_string_lossy().into_owned());
            }
        }
    }
    let dev = Path::new("resources/arena_config.json");
    if dev.is_file() {
        return Some(dev.to_string_lossy().into_owned());
    }
    None
}

fn load() -> &'static ArenaConfig {
    CONFIG.get_or_init(|| {
        let path = find_config_path().expect("arena_config.json not found");
        let raw = std::fs::read_to_string(&path).expect("read arena_config.json");
        let root: Value = serde_json::from_str(&raw).expect("parse arena_config.json");
        let pb = &root["playbook_names"];
        ArenaConfig {
            calibration_validate_path: root["calibration_validate_path"]
                .as_str()
                .expect("calibration_validate_path")
                .to_string(),
            calibration_api_happy_path_id: pb["calibration_api_happy_path"]
                .as_str()
                .expect("calibration_api_happy_path")
                .to_string(),
            calibration_api_error_path_id: pb["calibration_api_error_path"]
                .as_str()
                .expect("calibration_api_error_path")
                .to_string(),
            calibration_api_flaky_path_id: pb["calibration_api_flaky_path"]
                .as_str()
                .expect("calibration_api_flaky_path")
                .to_string(),
            reset_validation_db_id: pb["validation_db_scoped"]
                .as_str()
                .expect("validation_db_scoped")
                .to_string(),
            events_purge_id: pb["events_purge"]
                .as_str()
                .expect("events_purge")
                .to_string(),
        }
    })
}

pub fn calibration_validate_path() -> &'static str {
    &load().calibration_validate_path
}

pub fn calibration_api_happy_path_id() -> &'static str {
    &load().calibration_api_happy_path_id
}

pub fn calibration_api_error_path_id() -> &'static str {
    &load().calibration_api_error_path_id
}

pub fn calibration_api_flaky_path_id() -> &'static str {
    &load().calibration_api_flaky_path_id
}

pub fn reset_validation_db_id() -> &'static str {
    &load().reset_validation_db_id
}

#[allow(dead_code)]
pub fn events_purge_id() -> &'static str {
    &load().events_purge_id
}
