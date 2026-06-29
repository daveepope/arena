use crate::features::Features;

const PROBE_WEIGHT: f64 = 3.0;
const SHELL_WEIGHT: f64 = 2.0;
const COMPLEXITY_WEIGHT: f64 = 1.0;
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
    (risk / mitigation).min(DEBT_CEILING)
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
    let debt = (risk / mitigation).min(DEBT_CEILING);
    let grade = grade(debt);
    Report {
        features,
        risk,
        mitigation,
        debt,
        grade,
    }
}

fn risk(features: &Features) -> f64 {
    let introduced = features.complexity_introduced.max(0) as f64;
    (features.spec_gap + COMPLEXITY_WEIGHT * introduced) * (1.0 + features.max_autonomy_run as f64)
}

fn mitigation(features: &Features) -> f64 {
    1.0 + PROBE_WEIGHT * features.probe_hits as f64
        + features.read_ops as f64
        + SHELL_WEIGHT * features.shell_ops as f64
}
