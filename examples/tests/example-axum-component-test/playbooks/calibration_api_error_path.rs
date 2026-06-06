use arena_http::{server_error, ManagedHttpPlaybook};

use super::ids::{calibration_api_error_path_id, calibration_validate_path};

pub fn calibration_api_error_path_playbook(
    calibration_id: impl Into<String>,
) -> ManagedHttpPlaybook {
    ManagedHttpPlaybook::new(calibration_api_error_path_id(), calibration_id, |pb| {
        pb.post(calibration_validate_path())
            .will_return(server_error())
            .into_playbook()
    })
}
