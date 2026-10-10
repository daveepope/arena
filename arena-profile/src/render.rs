use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;
use inferno::flamegraph::color::{BackgroundColor, Color};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::Path;
use std::process::Command;

const ARENA_LOGO_JPEG: &[u8] = include_bytes!("../../arena-logo.png");
const PAGE_BACKGROUND: &str = "#121212";
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
         .arena-profile-page {{ width:100%; margin:0; padding:0 24px; box-sizing:border-box; }}\n\
         .arena-profile-header {{ display:flex; justify-content:flex-end; padding:8px 0; }}\n\
         .arena-profile-header img {{ height:40px; }}\n\
         svg {{ display:block; width:100%; height:auto; }}\n\
         </style>\n\
         </head>\n<body>\n\
         <div class=\"arena-profile-page\">\n\
         <div class=\"arena-profile-header\"><img src=\"data:image/jpeg;base64,{logo}\" alt=\"Arena\"></div>\n",
        page_bg = PAGE_BACKGROUND,
        logo = BASE64.encode(ARENA_LOGO_JPEG),
    )?;

    write_toolbar_style(&mut html, UI_TEXT_COLOR)?;
    write_toolbar_controls(&mut html, include_hotspots)?;

    if include_hotspots {
        write!(
            html,
            "<div class=\"arena-profile-panels\" id=\"arena-profile-panels\">\n\
             <div class=\"arena-profile-panel arena-profile-panel-graph\" id=\"arena-profile-panel-graph\">\n",
        )?;
    }

    let mut opts = inferno::flamegraph::Options::default();
    opts.colors = inferno::flamegraph::color::Palette::Basic(inferno::flamegraph::color::BasicPalette::Blue);
    opts.bgcolors = Some(BackgroundColor::Flat(FLAMEGRAPH_BACKGROUND));
    opts.uicolor = UI_TEXT_COLOR;
    inferno::flamegraph::from_reader(&mut opts, folded_text.as_bytes(), &mut html)
        .map_err(|e| RenderError::Inferno(e.to_string()))?;

    if include_hotspots {
        write!(
            html,
            "</div>\n\
             <div class=\"arena-profile-panel-divider\" id=\"arena-profile-panel-divider\"></div>\n\
             <div class=\"arena-profile-panel arena-profile-panel-hotspots\" id=\"arena-profile-panel-hotspots\">\n",
        )?;
        write_hotspots_style(&mut html, UI_TEXT_COLOR)?;
        write_hotspots_table(&mut html, &top_hotspots(&folded_text, HOTSPOT_LIMIT))?;
        write!(html, "</div>\n</div>\n")?;
    }

    write_toolbar_script(&mut html, include_hotspots)?;

    write!(html, "\n</div>\n</body>\n</html>\n")?;
    Ok(())
}

fn write_toolbar_style(html: &mut impl Write, ui_color: Color) -> std::io::Result<()> {
    write!(
        html,
        "<style>\n\
         .arena-profile-controls {{ display:flex; align-items:center; gap:8px; margin:8px 0; font-family:sans-serif; color:{ui_color}; font-size:13px; }}\n\
         .arena-profile-controls button {{ background:#2a2a2a; color:{ui_color}; border:1px solid #444; border-radius:4px; padding:4px 10px; cursor:pointer; font-size:13px; }}\n\
         .arena-profile-controls button.active {{ background:#3a6ea5; border-color:#3a6ea5; color:#fff; }}\n\
         .arena-profile-controls input[type=text] {{ background:#1a1a1a; color:{ui_color}; border:1px solid #444; border-radius:4px; padding:4px 8px; font-size:13px; width:280px; }}\n\
         .arena-profile-filtered-out {{ opacity:0.12; }}\n\
         .arena-profile-panels {{ display:flex; flex-direction:column; height:80vh; }}\n\
         .arena-profile-panels.side-by-side {{ flex-direction:row; }}\n\
         .arena-profile-panel {{ overflow:auto; min-width:0; min-height:0; }}\n\
         .arena-profile-panel-hotspots {{ order:1; }}\n\
         .arena-profile-panel-divider {{ order:2; flex:0 0 auto; background:#333; }}\n\
         .arena-profile-panel-graph {{ order:3; }}\n\
         .arena-profile-panel-divider:hover {{ background:#555; }}\n\
         .arena-profile-panels:not(.side-by-side) > .arena-profile-panel-divider {{ height:6px; cursor:row-resize; }}\n\
         .arena-profile-panels.side-by-side > .arena-profile-panel-divider {{ width:6px; cursor:col-resize; }}\n\
         </style>\n",
    )
}

fn write_toolbar_controls(html: &mut impl Write, include_layout_toggle: bool) -> std::io::Result<()> {
    write!(
        html,
        "<div class=\"arena-profile-controls\">\n\
         <span>Filter:</span>\n\
         <input type=\"text\" id=\"arena-profile-filter-input\" placeholder=\"e.g. your module or package path (regexp allowed)\">\n\
         <button type=\"button\" id=\"arena-profile-filter-clear\">Clear</button>\n",
    )?;
    if include_layout_toggle {
        write!(
            html,
            "<span>Layout:</span>\n\
             <button type=\"button\" id=\"arena-profile-layout-stacked\">Stacked</button>\n\
             <button type=\"button\" id=\"arena-profile-layout-side\">Side by side</button>\n",
        )?;
    }
    write!(html, "</div>\n")
}

fn write_toolbar_script(html: &mut impl Write, include_hotspots: bool) -> std::io::Result<()> {
    write!(
        html,
        "<script>\n\
         (function () {{\n\
         function escapeRegExp(s) {{ return s.replace(/[.*+?^${{}}()|[\\]\\\\]/g, \"\\\\$&\"); }}\n",
    )?;

    if include_hotspots {
        write!(
            html,
            "document.querySelectorAll(\".arena-profile-hotspots tbody tr\").forEach(function (row) {{\n\
             row.addEventListener(\"mouseenter\", function () {{\n\
             if (typeof search !== \"function\") return;\n\
             search(\"^\" + escapeRegExp(row.dataset.fn) + \" \\\\(\");\n\
             }});\n\
             row.addEventListener(\"mouseleave\", function () {{\n\
             if (typeof reset_search !== \"function\") return;\n\
             reset_search();\n\
             searching = 0;\n\
             if (typeof searchbtn !== \"undefined\" && searchbtn) {{\n\
             searchbtn.classList.remove(\"show\");\n\
             searchbtn.firstChild.nodeValue = \"Search\";\n\
             }}\n\
             if (typeof matchedtxt !== \"undefined\" && matchedtxt) {{\n\
             matchedtxt.classList.add(\"hide\");\n\
             matchedtxt.firstChild.nodeValue = \"\";\n\
             }}\n\
             }});\n\
             }});\n",
        )?;
    }

    write!(
        html,
        "var filterInput = document.getElementById(\"arena-profile-filter-input\");\n\
         var filterClear = document.getElementById(\"arena-profile-filter-clear\");\n\
         function applyFilter() {{\n\
         var term = filterInput.value.trim();\n\
         var re = null;\n\
         if (term) {{\n\
         try {{ re = new RegExp(term); }} catch (e) {{ re = null; }}\n\
         }}\n\
         if (typeof frames !== \"undefined\" && frames) {{\n\
         var frameEls = frames.children;\n\
         for (var i = 0; i < frameEls.length; i++) {{\n\
         var el = frameEls[i];\n\
         var func = (typeof g_to_func === \"function\") ? g_to_func(el) : null;\n\
         var frameMatches = !re || (func && re.test(func));\n\
         el.classList.toggle(\"arena-profile-filtered-out\", !frameMatches);\n\
         }}\n\
         }}\n",
    )?;

    if include_hotspots {
        write!(
            html,
            "document.querySelectorAll(\".arena-profile-hotspots tbody tr\").forEach(function (row) {{\n\
             var fn = row.dataset.fn || \"\";\n\
             var rowMatches = !re || re.test(fn);\n\
             row.classList.toggle(\"arena-profile-filtered-out\", !rowMatches);\n\
             }});\n",
        )?;
    }

    write!(
        html,
        "}}\n\
         if (filterInput) {{\n\
         filterInput.addEventListener(\"input\", applyFilter);\n\
         }}\n\
         if (filterClear) {{\n\
         filterClear.addEventListener(\"click\", function () {{\n\
         filterInput.value = \"\";\n\
         applyFilter();\n\
         }});\n\
         }}\n\
         var panels = document.getElementById(\"arena-profile-panels\");\n\
         if (panels) {{\n\
         var graph = document.getElementById(\"arena-profile-panel-graph\");\n\
         var hotspotsPanel = document.getElementById(\"arena-profile-panel-hotspots\");\n\
         var divider = document.getElementById(\"arena-profile-panel-divider\");\n\
         var stackedBtn = document.getElementById(\"arena-profile-layout-stacked\");\n\
         var sideBtn = document.getElementById(\"arena-profile-layout-side\");\n\
         var applySplit = function (pct) {{\n\
         hotspotsPanel.style.flex = \"0 0 \" + pct + \"%\";\n\
         graph.style.flex = \"0 0 \" + (100 - pct) + \"%\";\n\
         }};\n\
         var setLayout = function (sideBySide) {{\n\
         panels.classList.toggle(\"side-by-side\", sideBySide);\n\
         stackedBtn.classList.toggle(\"active\", !sideBySide);\n\
         sideBtn.classList.toggle(\"active\", sideBySide);\n\
         applySplit(40);\n\
         }};\n\
         stackedBtn.addEventListener(\"click\", function () {{ setLayout(false); }});\n\
         sideBtn.addEventListener(\"click\", function () {{ setLayout(true); }});\n\
         var dragging = false;\n\
         divider.addEventListener(\"mousedown\", function (e) {{\n\
         dragging = true;\n\
         e.preventDefault();\n\
         }});\n\
         document.addEventListener(\"mousemove\", function (e) {{\n\
         if (!dragging) return;\n\
         var rect = panels.getBoundingClientRect();\n\
         var pct;\n\
         if (panels.classList.contains(\"side-by-side\")) {{\n\
         pct = 100 * (e.clientX - rect.left) / rect.width;\n\
         }} else {{\n\
         pct = 100 * (e.clientY - rect.top) / rect.height;\n\
         }}\n\
         pct = Math.min(85, Math.max(15, pct));\n\
         applySplit(pct);\n\
         }});\n\
         document.addEventListener(\"mouseup\", function () {{ dragging = false; }});\n\
         setLayout(true);\n\
         }}\n\
         }})();\n\
         </script>\n",
    )
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
            Severity::High => "#b94a3a",
            Severity::Medium => "#8d4038",
            Severity::Low => "#5c3530",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Severity::Critical => "20%+ of captured samples",
            Severity::High => "10-20% of captured samples",
            Severity::Medium => "5-10% of captured samples",
            Severity::Low => "Under 5% of captured samples",
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
         .arena-profile-hotspots tbody tr {{ cursor:pointer; }}\n\
         .arena-profile-hotspots tbody tr:hover {{ background:rgba(255,255,255,0.06); }}\n\
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
            "<tr data-fn=\"{function_attr}\">\
             <td style=\"border-left:4px solid {color}\">{rank}</td><td>{function}</td><td>{count}</td><td>{pct:.1}%</td>\
             <td><span class=\"severity-badge\" style=\"background:{color}\">{severity}</span></td>\
             <td>{reason}</td></tr>\n",
            rank = rank + 1,
            function_attr = html_escape(&hotspot.function),
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
