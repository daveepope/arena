use arena_http::{ok_json, ManagedHttpPlaybook};
use serde_json::json;

use super::ids::{calibration_default_id, calibration_validate_path};

pub fn calibration_default_playbook(calibration_id: impl Into<String>) -> ManagedHttpPlaybook {
    ManagedHttpPlaybook::new(calibration_default_id(), calibration_id, |pb| {
        pb.post(calibration_validate_path())
            .will_return(ok_json(json!({ "valid": true })))
            .expect_called_at_least(1)
            .into_playbook()
    })
}
