use super::{map_spawn_error, resolve_binary, scratch_path, signal_interrupt, wait_bounded};
use crate::profiler::{CpuProfileError, LaunchRequest};
use crate::wrapped::{WrapState, WrappingSampler};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

const DOTNET_TRACE_RLOCATIONS: &[&str] = &["_main/tools/dotnet_trace/dotnet_trace_tool.sh"];
const DOTNET_TRACE_PATH_FALLBACK: &str = "dotnet-trace";
const INSTALL_HINT: &str = "install the dotnet-trace global tool: `dotnet tool install -g dotnet-trace`";

pub struct DotnetTraceSampler;

pub fn record_args(nettrace_path: &Path, request: &LaunchRequest) -> Vec<String> {
    let mut args: Vec<String> = vec![
        "collect".into(),
        "-o".into(),
        nettrace_path.to_string_lossy().into_owned(),
        "--".into(),
    ];
    args.push(request.program.to_string_lossy().into_owned());
    args.extend(request.args.iter().cloned());
    args
}

impl WrappingSampler for DotnetTraceSampler {
    fn wrap(
        &self,
        request: &LaunchRequest,
        _output_path: &Path,
    ) -> Result<(PathBuf, Vec<String>, WrapState), CpuProfileError> {
        let dotnet_trace =
            resolve_binary(DOTNET_TRACE_RLOCATIONS, DOTNET_TRACE_PATH_FALLBACK, DOTNET_TRACE_PATH_FALLBACK, INSTALL_HINT)?;
        let nettrace_path = scratch_path("arena-profile-dotnet-trace", "nettrace");
        let args = record_args(&nettrace_path, request);

        Ok((PathBuf::from(dotnet_trace), args, WrapState::DotnetTrace { nettrace_path }))
    }

    fn collect(
        &self,
        state: WrapState,
        wrapping_child: &mut Child,
        budget: Duration,
    ) -> Result<PathBuf, CpuProfileError> {
        let WrapState::DotnetTrace { nettrace_path } = state else {
            unreachable!("DotnetTraceSampler::collect given a non-dotnet-trace WrapState");
        };

        signal_interrupt(wrapping_child)
            .map_err(|e| CpuProfileError::Finish(format!("failed to signal dotnet-trace collect: {e}")))?;
        wait_bounded(wrapping_child, budget)
            .map_err(|e| CpuProfileError::Finish(format!("dotnet-trace collect did not exit cleanly: {e}")))?;

        // dotnet-trace's own exit code mirrors the traced process's exit code, not whether the
        // trace itself succeeded, so success is judged by whether it actually wrote the file.
        if !nettrace_path.exists() {
            return Err(CpuProfileError::Finish(format!(
                "dotnet-trace did not write a trace file at {}",
                nettrace_path.display()
            )));
        }

        convert_nettrace_to_folded(&nettrace_path)
    }
}

fn convert_nettrace_to_folded(nettrace_path: &Path) -> Result<PathBuf, CpuProfileError> {
    let dotnet_trace =
        resolve_binary(DOTNET_TRACE_RLOCATIONS, DOTNET_TRACE_PATH_FALLBACK, DOTNET_TRACE_PATH_FALLBACK, INSTALL_HINT)?;
    let convert_base = nettrace_path.with_extension("");

    let output = Command::new(&dotnet_trace)
        .args(["convert", &nettrace_path.to_string_lossy(), "--format", "Chromium", "-o"])
        .arg(&convert_base)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| map_spawn_error(DOTNET_TRACE_PATH_FALLBACK, INSTALL_HINT, e))?;
    if !output.status.success() {
        return Err(CpuProfileError::Finish(format!(
            "dotnet-trace convert failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    let mut chromium_path = convert_base.into_os_string();
    chromium_path.push(".chromium.json");
    let chromium_path = PathBuf::from(chromium_path);

    let chromium_json = std::fs::read_to_string(&chromium_path).map_err(|e| {
        CpuProfileError::Finish(format!("failed to read dotnet-trace chromium output: {e}"))
    })?;
    let folded_text = chromium_trace_to_folded(&chromium_json)
        .map_err(|e| CpuProfileError::Finish(format!("failed to fold dotnet-trace chromium output: {e}")))?;

    let folded_path = scratch_path("arena-profile-dotnet-trace", "folded");
    std::fs::write(&folded_path, folded_text)
        .map_err(|e| CpuProfileError::Finish(format!("failed to write folded stacks: {e}")))?;

    let _ = std::fs::remove_file(nettrace_path);
    let _ = std::fs::remove_file(&chromium_path);

    Ok(folded_path)
}

#[derive(serde::Deserialize)]
struct ChromiumTrace {
    #[serde(rename = "traceEvents")]
    trace_events: Vec<ChromiumEvent>,
    #[serde(rename = "stackFrames")]
    stack_frames: HashMap<String, ChromiumFrame>,
}

#[derive(serde::Deserialize)]
struct ChromiumEvent {
    ph: String,
    ts: f64,
    tid: u64,
    sf: Option<u64>,
}

#[derive(serde::Deserialize)]
struct ChromiumFrame {
    name: String,
    parent: Option<u64>,
}

fn frame_stack_line(stack_frames: &HashMap<String, ChromiumFrame>, leaf: u64) -> String {
    let mut names = Vec::new();
    let mut current = Some(leaf);
    while let Some(id) = current {
        let Some(frame) = stack_frames.get(&id.to_string()) else {
            break;
        };
        names.push(frame.name.replace(';', ":").replace('\n', " "));
        current = frame.parent;
    }
    names.reverse();
    names.join(";")
}

pub fn chromium_trace_to_folded(json: &str) -> Result<String, String> {
    let trace: ChromiumTrace = serde_json::from_str(json).map_err(|e| e.to_string())?;

    let mut by_thread: HashMap<u64, Vec<&ChromiumEvent>> = HashMap::new();
    for event in &trace.trace_events {
        if event.ph == "B" || event.ph == "E" {
            by_thread.entry(event.tid).or_default().push(event);
        }
    }

    let mut weights: HashMap<String, u64> = HashMap::new();
    for events in by_thread.values_mut() {
        events.sort_by(|a, b| a.ts.partial_cmp(&b.ts).unwrap_or(std::cmp::Ordering::Equal));
        let mut stack: Vec<u64> = Vec::new();
        let mut last_ts: Option<f64> = None;
        for event in events.iter() {
            if let (Some(prev_ts), Some(&top)) = (last_ts, stack.last()) {
                let duration = event.ts - prev_ts;
                if duration > 0.0 {
                    let line = frame_stack_line(&trace.stack_frames, top);
                    if !line.is_empty() {
                        *weights.entry(line).or_insert(0) += duration.round().max(1.0) as u64;
                    }
                }
            }
            match event.ph.as_str() {
                "B" => {
                    if let Some(sf) = event.sf {
                        stack.push(sf);
                    }
                }
                "E" => {
                    stack.pop();
                }
                _ => {}
            }
            last_ts = Some(event.ts);
        }
    }

    if weights.is_empty() {
        return Err("no stack samples found in dotnet-trace chromium output".to_string());
    }

    let mut lines: Vec<String> = weights.into_iter().map(|(stack, count)| format!("{stack} {count}")).collect();
    lines.sort();
    Ok(lines.join("\n") + "\n")
}
