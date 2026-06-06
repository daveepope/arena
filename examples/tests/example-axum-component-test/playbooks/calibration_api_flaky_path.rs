use arena_http::{ok_json, server_error, status, ManagedHttpPlaybook};
use serde_json::json;

use super::ids::{calibration_api_flaky_path_id, calibration_validate_path};

pub fn calibration_api_flaky_path_playbook(
    calibration_id: impl Into<String>,
) -> ManagedHttpPlaybook {
    ManagedHttpPlaybook::new(calibration_api_flaky_path_id(), calibration_id, |pb| {
        pb.post(calibration_validate_path())
            .will_return(server_error())
            .then_return(status(503))
            .then_return(ok_json(json!({ "valid": true })))
            .into_playbook()
    })
}
