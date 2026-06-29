use crate::features::Features;

const BYTES_WEIGHT: f64 = 1.0 / 512.0;
const FILES_WEIGHT: f64 = 3.0;
const COMPLEXITY_WEIGHT: f64 = 1.0;
const PROBE_WEIGHT: f64 = 3.0;
const READ_CAP: usize = 10;
const READ_WEIGHT: f64 = 0.25;
const SHELL_WEIGHT: f64 = 2.0;
const DEBT_CEILING: f64 = 100.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Grade {
    Low,
    Moderate,
    High,
    Severe,
}

impl Grade {
    pub fn label(self) -> &'static str {
        match self {
            Grade::Low => "low",
            Grade::Moderate => "moderate",
            Grade::High => "high",
            Grade::Severe => "severe",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Report {
    pub features: Features,
    pub risk: f64,
    pub mitigation: f64,
    pub debt: f64,
    pub grade: Grade,
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
pub struct DebtSample {
    pub at_ms: u64,
    pub debt: f64,
}

pub fn score(features: &Features) -> f64 {
    let risk = risk(features);
    let mitigation = mitigation(features);
    (risk / mitigation).clamp(0.0, DEBT_CEILING)
}

pub fn grade(debt: f64) -> Grade {
    if debt < 10.0 {
        Grade::Low
    } else if debt < 30.0 {
        Grade::Moderate
    } else if debt < 60.0 {
        Grade::High
    } else {
        Grade::Severe
    }
}

pub fn report(features: Features) -> Report {
    let risk = risk(&features);
    let mitigation = mitigation(&features);
    let debt = (risk / mitigation).clamp(0.0, DEBT_CEILING);
    let grade = grade(debt);
    Report {
        features,
        risk,
        mitigation,
        debt,
        grade,
    }
}

fn volume(features: &Features) -> f64 {
    BYTES_WEIGHT * features.bytes_delta as f64
        + FILES_WEIGHT * features.files_delta as f64
        + COMPLEXITY_WEIGHT * features.complexity_introduced as f64
}

fn risk(features: &Features) -> f64 {
    volume(features).max(0.0) * (1.0 + features.max_autonomy_run as f64)
}

fn mitigation(features: &Features) -> f64 {
    1.0
        + PROBE_WEIGHT * features.probe_hits as f64
        + READ_WEIGHT * features.read_ops.min(READ_CAP) as f64
        + SHELL_WEIGHT * features.shell_ops as f64
}
