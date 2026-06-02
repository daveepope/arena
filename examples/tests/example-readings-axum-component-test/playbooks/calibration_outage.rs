use arena_http::{server_error, ManagedHttpPlaybook};

use super::ids::{calibration_outage_managed_id, calibration_validate_path};

pub fn calibration_outage_playbook(calibration_id: impl Into<String>) -> ManagedHttpPlaybook {
    ManagedHttpPlaybook::new(calibration_outage_managed_id(), calibration_id, |pb| {
        pb.post(calibration_validate_path())
            .will_return(server_error())
            .expect_called_at_least(1)
            .into_playbook()
    })
}
