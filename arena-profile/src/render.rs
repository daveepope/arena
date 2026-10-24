use inferno::flamegraph::color::{BackgroundColor, Color};
use std::collections::HashMap;

const FLAMEGRAPH_BACKGROUND: Color = Color { r: 0x1e, g: 0x1e, b: 0x1e };
const UI_TEXT_COLOR: Color = Color { r: 0xdd, g: 0xdd, b: 0xdd };
pub const HOTSPOT_LIMIT: usize = 10;

#[derive(Debug)]
pub enum RenderError {
    Io(std::io::Error),
    Inferno(String),
    NotUtf8(std::string::FromUtf8Error),
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RenderError::Io(e) => write!(f, "io error rendering flamegraph: {e}"),
            RenderError::Inferno(msg) => write!(f, "failed to render flamegraph: {msg}"),
            RenderError::NotUtf8(e) => write!(f, "rendered flamegraph was not utf-8: {e}"),
        }
    }
}

impl std::error::Error for RenderError {}

impl From<std::io::Error> for RenderError {
    fn from(e: std::io::Error) -> Self {
        RenderError::Io(e)
    }
}

impl From<std::string::FromUtf8Error> for RenderError {
    fn from(e: std::string::FromUtf8Error) -> Self {
        RenderError::NotUtf8(e)
    }
}

pub struct RenderedProfile {
    pub svg: String,
    pub hotspots: Vec<Hotspot>,
    pub application_hotspots: Vec<Hotspot>,
}

pub struct RuntimeFrames {
    pub language: &'static str,
    pub starts_with: &'static [&'static str],
    pub contains: &'static [&'static str],
}

pub const RUNTIME_FRAMES: &[RuntimeFrames] = &[
    RuntimeFrames {
        language: "dotnet",
        starts_with: &[
            "UNMANAGED_CODE_TIME",
            "CPU_TIME",
            "Thread (",
            "System.",
            "Microsoft.",
            "Internal.",
            "Interop",
        ],
        contains: &[],
    },
    RuntimeFrames {
        language: "java",
        starts_with: &[
            "java.",
            "javax.",
            "jdk.",
            "sun.",
            "org.springframework.",
            "org.apache.",
            "JVM_",
            "Phase",
            "Compile::",
            "IndexSet",
            "SymbolTable::",
            "JavaThread::",
            "SharedRuntime::",
            "InterpreterRuntime::",
            "Universe::",
            "G1",
            "ciMethod",
            "ciEnv",
            "OopMap",
            "CodeCache",
            "Monitor::",
            "os::",
            "Node::",
            "GraphKit::",
        ],
        contains: &[],
    },
    RuntimeFrames {
        language: "python",
        starts_with: &[],
        contains: &[
            "(<frozen importlib",
            "(importlib/",
            "(asyncio/",
            "(concurrent/futures/",
            "(encodings/",
            "(logging/",
            "(json/",
            "(re/",
            "(threading.py",
            "(socket.py",
            "(ssl.py",
            "(selectors.py",
            "(dataclasses.py",
            "(typing.py",
            "(functools.py",
            "(abc.py",
            "(enum.py",
        ],
    },
    RuntimeFrames {
        language: "native",
        starts_with: &[
            "[unknown",
            "/usr/lib/",
            "/lib/",
            "__tls_get_addr",
            "__memset",
            "__memcpy",
            "__memmove",
            "__pthread",
            "__libc",
            "std::",
            "core::",
            "tokio::",
        ],
        contains: &[],
    },
];

pub fn is_runtime_frame(function: &str) -> bool {
    RUNTIME_FRAMES.iter().any(|group| {
        group
            .starts_with
            .iter()
            .any(|prefix| function.starts_with(prefix))
            || group
                .contains
                .iter()
                .any(|marker| function.contains(marker))
    })
}

pub fn render_folded(folded: &str) -> Result<RenderedProfile, RenderError> {
    let mut svg = Vec::new();
    let mut opts = inferno::flamegraph::Options::default();
    opts.colors =
        inferno::flamegraph::color::Palette::Basic(inferno::flamegraph::color::BasicPalette::Blue);
    opts.bgcolors = Some(BackgroundColor::Flat(FLAMEGRAPH_BACKGROUND));
    opts.uicolor = UI_TEXT_COLOR;
    inferno::flamegraph::from_reader(&mut opts, folded.as_bytes(), &mut svg)
        .map_err(|e| RenderError::Inferno(e.to_string()))?;

    Ok(RenderedProfile {
        svg: String::from_utf8(svg)?,
        hotspots: top_hotspots(folded, HOTSPOT_LIMIT),
        application_hotspots: top_application_hotspots(folded, HOTSPOT_LIMIT),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

impl Severity {
    const CRITICAL_THRESHOLD_PCT: f64 = 20.0;
    const HIGH_THRESHOLD_PCT: f64 = 10.0;
    const MEDIUM_THRESHOLD_PCT: f64 = 5.0;

    pub fn for_self_pct(self_pct: f64) -> Self {
        if self_pct >= Self::CRITICAL_THRESHOLD_PCT {
            Severity::Critical
        } else if self_pct >= Self::HIGH_THRESHOLD_PCT {
            Severity::High
        } else if self_pct >= Self::MEDIUM_THRESHOLD_PCT {
            Severity::Medium
        } else {
            Severity::Low
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Severity::Critical => "Critical",
            Severity::High => "High",
            Severity::Medium => "Medium",
            Severity::Low => "Low",
        }
    }

    pub fn badge_color(self) -> &'static str {
        match self {
            Severity::Critical => "#e74c3c",
            Severity::High => "#b94a3a",
            Severity::Medium => "#8d4038",
            Severity::Low => "#5c3530",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Severity::Critical => "20%+ of captured samples",
            Severity::High => "10-20% of captured samples",
            Severity::Medium => "5-10% of captured samples",
            Severity::Low => "Under 5% of captured samples",
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Hotspot {
    pub function: String,
    pub self_count: u64,
    pub self_pct: f64,
    pub severity: Severity,
}

pub fn top_application_hotspots(folded: &str, limit: usize) -> Vec<Hotspot> {
    ranked_hotspots(folded, limit, true)
}

pub fn top_hotspots(folded: &str, limit: usize) -> Vec<Hotspot> {
    ranked_hotspots(folded, limit, false)
}

fn ranked_hotspots(folded: &str, limit: usize, skip_runtime_frames: bool) -> Vec<Hotspot> {
    let mut self_counts: HashMap<&str, u64> = HashMap::new();
    let mut total: u64 = 0;

    for line in folded.lines() {
        let line = line.trim();
        let Some((stack, count)) = line.rsplit_once(' ') else {
            continue;
        };
        let Ok(count) = count.parse::<u64>() else {
            continue;
        };
        total += count;
        let leaf = stack.rsplit(';').next().unwrap_or(stack);
        let credited = if skip_runtime_frames {
            match stack.rsplit(';').find(|frame| !is_runtime_frame(frame)) {
                Some(frame) => frame,
                None => continue,
            }
        } else {
            leaf
        };
        *self_counts.entry(credited).or_insert(0) += count;
    }

    let mut hotspots: Vec<Hotspot> = self_counts
        .into_iter()
        .map(|(function, self_count)| {
            let self_pct = if total == 0 {
                0.0
            } else {
                100.0 * self_count as f64 / total as f64
            };
            Hotspot {
                function: function.to_string(),
                self_count,
                self_pct,
                severity: Severity::for_self_pct(self_pct),
            }
        })
        .collect();
    hotspots
        .sort_by(|a, b| b.self_count.cmp(&a.self_count).then_with(|| a.function.cmp(&b.function)));
    hotspots.truncate(limit);
    hotspots
}
