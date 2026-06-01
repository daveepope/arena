use serde_json::Value;
use std::path::Path;
use std::sync::OnceLock;

struct ReadingsArenaConfig {
    calibration_validate_path: String,
    calibration_default_id: String,
    calibration_outage_managed_id: String,
    reset_validation_db_id: String,
#[allow(dead_code)]
    localstack_session_id: String,
}

static CONFIG: OnceLock<ReadingsArenaConfig> = OnceLock::new();

fn find_config_path() -> Option<String> {
    if let Ok(runfiles) = std::env::var("RUNFILES_DIR") {
        for base in ["_main", "arena", ""] {
            let path = if base.is_empty() {
                Path::new(&runfiles).join("examples/resources/readings_arena_config.json")
            } else {
                Path::new(&runfiles)
                    .join(base)
                    .join("examples/resources/readings_arena_config.json")
            };
            if path.is_file() {
                return Some(path.to_string_lossy().into_owned());
            }
        }
    }
    let dev = Path::new("examples/resources/readings_arena_config.json");
    if dev.is_file() {
        return Some(dev.to_string_lossy().into_owned());
    }
    None
}

fn load() -> &'static ReadingsArenaConfig {
    CONFIG.get_or_init(|| {
        let path = find_config_path().expect("readings_arena_config.json not found");
        let raw = std::fs::read_to_string(&path).expect("read readings_arena_config.json");
        let root: Value = serde_json::from_str(&raw).expect("parse readings_arena_config.json");
        let pb = &root["playbook_names"];
        ReadingsArenaConfig {
            calibration_validate_path: root["calibration_validate_path"]
                .as_str()
                .expect("calibration_validate_path")
                .to_string(),
            calibration_default_id: pb["calibration_default"]
                .as_str()
                .expect("calibration_default")
                .to_string(),
            calibration_outage_managed_id: pb["calibration_outage_managed"]
                .as_str()
                .expect("calibration_outage_managed")
                .to_string(),
            reset_validation_db_id: pb["validation_db_scoped"]
                .as_str()
                .expect("validation_db_scoped")
                .to_string(),
            localstack_session_id: pb["localstack_session"]
                .as_str()
                .expect("localstack_session")
                .to_string(),
        }
    })
}

pub fn calibration_validate_path() -> &'static str {
    &load().calibration_validate_path
}

pub fn calibration_default_id() -> &'static str {
    &load().calibration_default_id
}

pub fn calibration_outage_managed_id() -> &'static str {
    &load().calibration_outage_managed_id
}

pub fn reset_validation_db_id() -> &'static str {
    &load().reset_validation_db_id
}

#[allow(dead_code)]
pub fn localstack_session_id() -> &'static str {
    &load().localstack_session_id
}
