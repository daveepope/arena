use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;
use inferno::flamegraph::color::{BackgroundColor, Color};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::Path;
use std::process::Command;

const ARENA_LOGO_JPEG: &[u8] = include_bytes!("../../arena-logo.png");
const PAGE_BACKGROUND: &str = "#121212";
const PAGE_MAX_WIDTH: u32 = 1400;
const FLAMEGRAPH_BACKGROUND: Color = Color { r: 0x1e, g: 0x1e, b: 0x1e };
const UI_TEXT_COLOR: Color = Color { r: 0xdd, g: 0xdd, b: 0xdd };
pub const HOTSPOT_LIMIT: usize = 10;

#[derive(Debug)]
pub enum RenderError {
    Io(std::io::Error),
    Inferno(String),
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RenderError::Io(e) => write!(f, "io error rendering flamegraph report: {e}"),
            RenderError::Inferno(msg) => write!(f, "failed to render flamegraph: {msg}"),
        }
    }
}

impl std::error::Error for RenderError {}

impl From<std::io::Error> for RenderError {
    fn from(e: std::io::Error) -> Self {
        RenderError::Io(e)
    }
}

pub fn render_folded_to_html(
    mut folded: impl Read,
    output_path: &Path,
    include_hotspots: bool,
) -> Result<(), RenderError> {
    let mut folded_text = String::new();
    folded.read_to_string(&mut folded_text)?;

    let mut html = std::fs::File::create(output_path)?;
    write!(
        html,
        "<!DOCTYPE html>\n<html>\n<head><meta charset=\"utf-8\"><title>CPU Profile</title>\n\
         <style>\n\
         html, body {{ margin:0; padding:0; background:{page_bg}; }}\n\
         .arena-profiler-page {{ max-width:{page_max_width}px; margin:0 auto; padding:0 24px; }}\n\
         .arena-profiler-header {{ display:flex; justify-content:flex-end; padding:8px 0; }}\n\
         .arena-profiler-header img {{ height:40px; }}\n\
         svg {{ display:block; width:100%; height:auto; }}\n\
         </style>\n\
         </head>\n<body>\n\
         <div class=\"arena-profiler-page\">\n\
         <div class=\"arena-profiler-header\"><img src=\"data:image/jpeg;base64,{logo}\" alt=\"Arena\"></div>\n",
        page_bg = PAGE_BACKGROUND,
        page_max_width = PAGE_MAX_WIDTH,
        logo = BASE64.encode(ARENA_LOGO_JPEG),
    )?;

    let mut opts = inferno::flamegraph::Options::default();
    opts.colors =
        inferno::flamegraph::color::Palette::Basic(inferno::flamegraph::color::BasicPalette::Blue);
    opts.bgcolors = Some(BackgroundColor::Flat(FLAMEGRAPH_BACKGROUND));
    opts.uicolor = UI_TEXT_COLOR;
    inferno::flamegraph::from_reader(&mut opts, folded_text.as_bytes(), &mut html)
        .map_err(|e| RenderError::Inferno(e.to_string()))?;

    if include_hotspots {
        write_hotspots_style(&mut html, UI_TEXT_COLOR)?;
        write_hotspots_table(&mut html, &top_hotspots(&folded_text, HOTSPOT_LIMIT))?;
    }

    write!(html, "\n</div>\n</body>\n</html>\n")?;
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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
            Severity::High => "#e67e22",
            Severity::Medium => "#f1c40f",
            Severity::Low => "#7f8c8d",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Severity::Critical => "20%+ of on-CPU samples",
            Severity::High => "10-20% of on-CPU samples",
            Severity::Medium => "5-10% of on-CPU samples",
            Severity::Low => "Under 5% of on-CPU samples",
        }
    }
}

pub struct Hotspot {
    pub function: String,
    pub self_count: u64,
    pub self_pct: f64,
    pub severity: Severity,
}

pub fn top_hotspots(folded: &str, limit: usize) -> Vec<Hotspot> {
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
        let leaf = stack.rsplit(';').next().unwrap_or(stack);
        *self_counts.entry(leaf).or_insert(0) += count;
        total += count;
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

fn write_hotspots_style(html: &mut impl Write, ui_color: Color) -> std::io::Result<()> {
    write!(
        html,
        "<style>\n\
         .arena-profiler-hotspots {{ margin:16px 0; font-family:sans-serif; color:{ui_color}; }}\n\
         .arena-profiler-hotspots table {{ border-collapse:collapse; width:100%; font-size:13px; }}\n\
         .arena-profiler-hotspots th, .arena-profiler-hotspots td {{ text-align:left; padding:4px 8px; border-bottom:1px solid #333; }}\n\
         .arena-profiler-hotspots .severity-badge {{ display:inline-block; padding:2px 8px; border-radius:3px; color:#111; font-weight:bold; }}\n\
         </style>\n",
    )
}

fn write_hotspots_table(html: &mut impl Write, hotspots: &[Hotspot]) -> std::io::Result<()> {
    write!(
        html,
        "<section class=\"arena-profiler-hotspots\">\n<h2>Top {} Hotspots (self time)</h2>\n\
         <table>\n<thead><tr><th>#</th><th>Function</th><th>Self Samples</th><th>Self %</th><th>Severity</th><th>Why</th></tr></thead>\n<tbody>\n",
        hotspots.len(),
    )?;
    for (rank, hotspot) in hotspots.iter().enumerate() {
        write!(
            html,
            "<tr><td>{rank}</td><td>{function}</td><td>{count}</td><td>{pct:.1}%</td>\
             <td><span class=\"severity-badge\" style=\"background:{color}\">{severity}</span></td>\
             <td>{reason}</td></tr>\n",
            rank = rank + 1,
            function = html_escape(&hotspot.function),
            count = hotspot.self_count,
            pct = hotspot.self_pct,
            color = hotspot.severity.badge_color(),
            severity = hotspot.severity.label(),
            reason = html_escape(hotspot.severity.description()),
        )?;
    }
    write!(html, "</tbody>\n</table>\n</section>\n")?;
    Ok(())
}

pub fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub fn open_report(path: &Path) -> std::io::Result<()> {
    if cfg!(target_os = "macos") {
        Command::new("open").arg(path).status()?;
        return Ok(());
    }
    if cfg!(target_os = "windows") {
        Command::new("cmd").args(["/C", "start", ""]).arg(path).status()?;
        return Ok(());
    }
    if is_wsl() {
        let win_path = Command::new("wslpath").arg("-w").arg(path).output()?;
        let win_path = String::from_utf8_lossy(&win_path.stdout).trim().to_string();
        // explorer.exe frequently returns a non-zero exit status even when it
        // successfully opens the file, so its status is intentionally not checked.
        let _ = Command::new("explorer.exe").arg(win_path).status();
        return Ok(());
    }
    Command::new("xdg-open").arg(path).status()?;
    Ok(())
}

pub fn is_wsl() -> bool {
    std::env::var_os("WSL_DISTRO_NAME").is_some()
        || std::fs::read_to_string("/proc/version")
            .map(|v| v.to_lowercase().contains("microsoft"))
            .unwrap_or(false)
}
