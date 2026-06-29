use crate::transcript::Event;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Features {
    pub user_turns: usize,
    pub assistant_turns: usize,
    pub prompt_chars: usize,
    pub edit_bytes: usize,
    pub read_ops: usize,
    pub shell_ops: usize,
    pub probe_hits: usize,
    pub autonomy_streak: usize,
    pub max_autonomy_run: usize,
    pub spec_gap: f64,
    pub bytes_delta: i64,
    pub files_delta: i64,
    pub complexity_introduced: i64,
}

pub fn extract(events: &[Event]) -> Features {
    let mut features = Features::default();
    let mut run = 0usize;

    for event in events {
        if event.role_user {
            features.user_turns += 1;
            features.prompt_chars += event.prompt_chars;
            features.probe_hits += event.probe_hits;
            run = 0;
        } else {
            features.assistant_turns += 1;
            run += 1;
            features.max_autonomy_run = features.max_autonomy_run.max(run);
        }
        features.edit_bytes += event.edit_bytes;
        features.read_ops += event.read_ops;
        features.shell_ops += event.shell_ops;
    }

    features.spec_gap = features.edit_bytes as f64 / features.prompt_chars.max(1) as f64;
    features.autonomy_streak = run;
    features
}
