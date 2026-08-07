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
const HOTSPOT_LIMIT: usize = 10;

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
         .arena-profile-page {{ max-width:{page_max_width}px; margin:0 auto; padding:0 24px; }}\n\
         .arena-profile-header {{ display:flex; justify-content:flex-end; padding:8px 0; }}\n\
         .arena-profile-header img {{ height:40px; }}\n\
         svg {{ display:block; width:100%; height:auto; }}\n\
         </style>\n\
         </head>\n<body>\n\
         <div class=\"arena-profile-page\">\n\
         <div class=\"arena-profile-header\"><img src=\"data:image/jpeg;base64,{logo}\" alt=\"Arena\"></div>\n",
        page_bg = PAGE_BACKGROUND,
        page_max_width = PAGE_MAX_WIDTH,
        logo = BASE64.encode(ARENA_LOGO_JPEG),
    )?;

    let mut opts = inferno::flamegraph::Options::default();
    opts.colors = inferno::flamegraph::color::Palette::Basic(inferno::flamegraph::color::BasicPalette::Blue);
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
enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

impl Severity {
    const CRITICAL_THRESHOLD_PCT: f64 = 20.0;
    const HIGH_THRESHOLD_PCT: f64 = 10.0;
    const MEDIUM_THRESHOLD_PCT: f64 = 5.0;

    fn for_self_pct(self_pct: f64) -> Self {
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

    fn label(self) -> &'static str {
        match self {
            Severity::Critical => "Critical",
            Severity::High => "High",
            Severity::Medium => "Medium",
            Severity::Low => "Low",
        }
    }

    fn badge_color(self) -> &'static str {
        match self {
            Severity::Critical => "#e74c3c",
            Severity::High => "#e67e22",
            Severity::Medium => "#f1c40f",
            Severity::Low => "#7f8c8d",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Severity::Critical => "20%+ of on-CPU samples",
            Severity::High => "10-20% of on-CPU samples",
            Severity::Medium => "5-10% of on-CPU samples",
            Severity::Low => "Under 5% of on-CPU samples",
        }
    }
}

struct Hotspot {
    function: String,
    self_count: u64,
    self_pct: f64,
    severity: Severity,
}

fn top_hotspots(folded: &str, limit: usize) -> Vec<Hotspot> {
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
    hotspots.sort_by(|a, b| b.self_count.cmp(&a.self_count).then_with(|| a.function.cmp(&b.function)));
    hotspots.truncate(limit);
    hotspots
}

fn write_hotspots_style(html: &mut impl Write, ui_color: Color) -> std::io::Result<()> {
    write!(
        html,
        "<style>\n\
         .arena-profile-hotspots {{ margin:16px 0; font-family:sans-serif; color:{ui_color}; }}\n\
         .arena-profile-hotspots table {{ border-collapse:collapse; width:100%; font-size:13px; }}\n\
         .arena-profile-hotspots th, .arena-profile-hotspots td {{ text-align:left; padding:4px 8px; border-bottom:1px solid #333; }}\n\
         .arena-profile-hotspots .severity-badge {{ display:inline-block; padding:2px 8px; border-radius:3px; color:#111; font-weight:bold; }}\n\
         </style>\n",
    )
}

fn write_hotspots_table(html: &mut impl Write, hotspots: &[Hotspot]) -> std::io::Result<()> {
    write!(
        html,
        "<section class=\"arena-profile-hotspots\">\n<h2>Top {} Hotspots (self time)</h2>\n\
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

fn html_escape(s: &str) -> String {
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

fn is_wsl() -> bool {
    std::env::var_os("WSL_DISTRO_NAME").is_some()
        || std::fs::read_to_string("/proc/version")
            .map(|v| v.to_lowercase().contains("microsoft"))
            .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_wsl_matches_wsl_distro_env_or_proc_version() {
        let expected = std::env::var_os("WSL_DISTRO_NAME").is_some()
            || std::fs::read_to_string("/proc/version")
                .map(|v| v.to_lowercase().contains("microsoft"))
                .unwrap_or(false);

        assert_eq!(is_wsl(), expected);
    }

    #[test]
    fn top_hotspots_ranks_by_leaf_self_time_descending() {
        let folded = "main;foo;compute 10\nmain;bar;compute 40\nmain;bar;idle 5\n";

        let hotspots = top_hotspots(folded, 10);

        assert_eq!(hotspots[0].function, "compute");
        assert_eq!(hotspots[0].self_count, 50);
        assert_eq!(hotspots[1].function, "idle");
        assert_eq!(hotspots[1].self_count, 5);
    }

    #[test]
    fn top_hotspots_computes_self_percentage_of_total_samples() {
        let folded = "main;a 3\nmain;b 1\n";

        let hotspots = top_hotspots(folded, 10);

        let a = hotspots.iter().find(|h| h.function == "a").unwrap();
        assert!((a.self_pct - 75.0).abs() < f64::EPSILON);
    }

    #[test]
    fn top_hotspots_limit_truncates_result() {
        let folded = "main;a 3\nmain;b 2\nmain;c 1\n";

        let hotspots = top_hotspots(folded, 2);

        assert_eq!(hotspots.len(), 2);
    }

    #[test]
    fn top_hotspots_empty_folded_returns_empty() {
        let hotspots = top_hotspots("", 10);

        assert!(hotspots.is_empty());
    }

    #[test]
    fn html_escape_reserved_characters_are_escaped() {
        assert_eq!(html_escape("a<b> && c"), "a&lt;b&gt; &amp;&amp; c");
    }

    #[test]
    fn severity_for_self_pct_below_medium_threshold_returns_low() {
        assert_eq!(Severity::for_self_pct(4.9), Severity::Low);
    }

    #[test]
    fn severity_for_self_pct_at_medium_threshold_returns_medium() {
        assert_eq!(Severity::for_self_pct(5.0), Severity::Medium);
    }

    #[test]
    fn severity_for_self_pct_at_high_threshold_returns_high() {
        assert_eq!(Severity::for_self_pct(10.0), Severity::High);
    }

    #[test]
    fn severity_for_self_pct_at_critical_threshold_returns_critical() {
        assert_eq!(Severity::for_self_pct(20.0), Severity::Critical);
    }

    #[test]
    fn top_hotspots_assigns_severity_from_self_pct() {
        let folded = "main;hot 93\nmain;cold 7\n";

        let hotspots = top_hotspots(folded, 10);

        let hot = hotspots.iter().find(|h| h.function == "hot").unwrap();
        let cold = hotspots.iter().find(|h| h.function == "cold").unwrap();
        assert_eq!(hot.severity, Severity::Critical);
        assert_eq!(cold.severity, Severity::Medium);
    }
}
