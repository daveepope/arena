use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use tree_sitter::{Node, Parser};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Language {
    Rust,
    Python,
    Java,
}

impl Language {
    pub fn from_extension(extension: &str) -> Option<Self> {
        match extension {
            "rs" => Some(Language::Rust),
            "py" => Some(Language::Python),
            "java" => Some(Language::Java),
            _ => None,
        }
    }

    pub fn from_path(path: &Path) -> Option<Self> {
        path.extension()
            .and_then(|extension| extension.to_str())
            .and_then(Self::from_extension)
    }

    fn grammar(self) -> tree_sitter::Language {
        match self {
            Language::Rust => tree_sitter_rust::LANGUAGE.into(),
            Language::Python => tree_sitter_python::LANGUAGE.into(),
            Language::Java => tree_sitter_java::LANGUAGE.into(),
        }
    }

    fn decision_kinds(self) -> &'static [&'static str] {
        match self {
            Language::Rust => &[
                "if_expression",
                "match_arm",
                "while_expression",
                "for_expression",
                "loop_expression",
                "&&",
                "||",
            ],
            Language::Python => &[
                "if_statement",
                "elif_clause",
                "for_statement",
                "while_statement",
                "except_clause",
                "conditional_expression",
                "case_clause",
                "and",
                "or",
            ],
            Language::Java => &[
                "if_statement",
                "while_statement",
                "for_statement",
                "enhanced_for_statement",
                "do_statement",
                "catch_clause",
                "switch_label",
                "ternary_expression",
                "&&",
                "||",
            ],
        }
    }
}

pub fn cyclomatic(source: &str, language: Language) -> u32 {
    let mut parser = Parser::new();
    if parser.set_language(&language.grammar()).is_err() {
        return 1;
    }
    let Some(tree) = parser.parse(source, None) else {
        return 1;
    };
    1 + count_decisions(tree.root_node(), language)
}

fn count_decisions(node: Node, language: Language) -> u32 {
    let mut total = if language.decision_kinds().contains(&node.kind()) {
        1
    } else {
        0
    };
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        total += count_decisions(child, language);
    }
    total
}

pub fn complexity_of(path: &Path) -> Option<u32> {
    let language = Language::from_path(path)?;
    let source = fs::read_to_string(path).ok()?;
    Some(cyclomatic(&source, language))
}

const IGNORED_DIRS: &[&str] = &[
    "target",
    "node_modules",
    "__pycache__",
    "dist",
    "build",
    "out",
];

pub fn collect_source_files(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(directory) = stack.pop() {
        let Ok(entries) = fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_dir() {
                if !is_ignored_dir(&path) {
                    stack.push(path);
                }
            } else if file_type.is_file() && Language::from_path(&path).is_some() {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

fn is_ignored_dir(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return true;
    };
    is_ignored_name(name)
}

fn is_ignored_name(name: &str) -> bool {
    name.starts_with('.') || name.starts_with("bazel-") || IGNORED_DIRS.contains(&name)
}

fn is_ignored_descendant(root: &Path, path: &Path) -> bool {
    let Ok(relative) = path.strip_prefix(root) else {
        return false;
    };
    relative.components().any(|component| {
        component
            .as_os_str()
            .to_str()
            .map(is_ignored_name)
            .unwrap_or(true)
    })
}

pub fn watchable_directories(root: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    let mut directories: Vec<PathBuf> = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let is_directory = entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false);
            if is_directory && !is_ignored_dir(&path) {
                Some(path)
            } else {
                None
            }
        })
        .collect();
    directories.sort();
    directories
}

pub fn baseline_complexity(root: &Path) -> HashMap<PathBuf, u32> {
    parallel_complexity(collect_source_files(root))
}

pub fn file_bytes(path: &Path) -> u64 {
    fs::metadata(path).map(|metadata| metadata.len()).unwrap_or(0)
}

pub fn baseline_bytes(root: &Path) -> HashMap<PathBuf, u64> {
    collect_source_files(root)
        .into_iter()
        .map(|path| (path.clone(), file_bytes(&path)))
        .collect()
}

pub fn bytes_delta(baseline: &HashMap<PathBuf, u64>, current: &HashMap<PathBuf, u64>) -> i64 {
    let baseline_total: i64 = baseline.values().map(|size| *size as i64).sum();
    let current_total: i64 = current.values().map(|size| *size as i64).sum();
    current_total - baseline_total
}

pub fn files_delta(baseline: &HashMap<PathBuf, u64>, current: &HashMap<PathBuf, u64>) -> i64 {
    current.len() as i64 - baseline.len() as i64
}

fn parallel_complexity(files: Vec<PathBuf>) -> HashMap<PathBuf, u32> {
    let workers = std::thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(1)
        .min(files.len());

    if workers <= 1 {
        return files
            .into_iter()
            .filter_map(|path| complexity_of(&path).map(|value| (path, value)))
            .collect();
    }

    let chunk_size = files.len().div_ceil(workers);
    let mut result = HashMap::with_capacity(files.len());
    std::thread::scope(|scope| {
        let handles: Vec<_> = files
            .chunks(chunk_size)
            .map(|chunk| {
                scope.spawn(move || {
                    chunk
                        .iter()
                        .filter_map(|path| complexity_of(path).map(|value| (path.clone(), value)))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        for handle in handles {
            if let Ok(partial) = handle.join() {
                result.extend(partial);
            }
        }
    });
    result
}

fn total(map: &HashMap<PathBuf, u32>) -> u32 {
    map.values().copied().sum()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComplexityDelta {
    pub path: PathBuf,
    pub baseline: u32,
    pub current: u32,
}

impl ComplexityDelta {
    pub fn delta(&self) -> i64 {
        i64::from(self.current) - i64::from(self.baseline)
    }
}

pub fn compute_deltas(
    baseline: &HashMap<PathBuf, u32>,
    current: &HashMap<PathBuf, u32>,
) -> Vec<ComplexityDelta> {
    let mut deltas: Vec<ComplexityDelta> = Vec::new();
    for (path, &current_value) in current {
        let baseline_value = baseline.get(path).copied().unwrap_or(0);
        if current_value != baseline_value {
            deltas.push(ComplexityDelta {
                path: path.clone(),
                baseline: baseline_value,
                current: current_value,
            });
        }
    }
    for (path, &baseline_value) in baseline {
        if !current.contains_key(path) {
            deltas.push(ComplexityDelta {
                path: path.clone(),
                baseline: baseline_value,
                current: 0,
            });
        }
    }
    deltas.sort_by(|left, right| {
        right
            .delta()
            .abs()
            .cmp(&left.delta().abs())
            .then_with(|| left.path.cmp(&right.path))
    });
    deltas
}

#[derive(Default)]
struct DiskState {
    baseline: HashMap<PathBuf, u32>,
    current: HashMap<PathBuf, u32>,
    baseline_bytes: HashMap<PathBuf, u64>,
    current_bytes: HashMap<PathBuf, u64>,
}

pub struct ComplexityEngine {
    root: PathBuf,
    state: Arc<Mutex<DiskState>>,
    watcher: Option<RecommendedWatcher>,
}

impl ComplexityEngine {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            state: Arc::new(Mutex::new(DiskState::default())),
            watcher: None,
        }
    }

    pub fn start(&mut self) -> notify::Result<()> {
        let baseline = baseline_complexity(&self.root);
        let baseline_bytes = baseline_bytes(&self.root);

        if let Ok(mut guard) = self.state.lock() {
            guard.current = baseline.clone();
            guard.baseline = baseline;
            guard.current_bytes = baseline_bytes.clone();
            guard.baseline_bytes = baseline_bytes;
        }

        let state = Arc::clone(&self.state);
        let root = self.root.clone();
        let mut watcher =
            notify::recommended_watcher(move |result: notify::Result<notify::Event>| {
                let Ok(event) = result else {
                    return;
                };
                for path in event.paths {
                    if is_ignored_descendant(&root, &path) {
                        continue;
                    }
                    let is_source = Language::from_path(&path).is_some();
                    let value = if is_source {
                        complexity_of(&path)
                    } else {
                        None
                    };
                    let size = if is_source && path.is_file() {
                        Some(file_bytes(&path))
                    } else {
                        None
                    };
                    if let Ok(mut guard) = state.lock() {
                        match value {
                            Some(value) => {
                                guard.current.insert(path.clone(), value);
                            }
                            None if is_source => {
                                guard.current.remove(&path);
                            }
                            None => {}
                        }
                        match size {
                            Some(size) => {
                                guard.current_bytes.insert(path, size);
                            }
                            None if is_source => {
                                guard.current_bytes.remove(&path);
                            }
                            None => {}
                        }
                    }
                }
            })?;

        let _ = watcher.watch(&self.root, RecursiveMode::NonRecursive);
        for directory in watchable_directories(&self.root) {
            let _ = watcher.watch(&directory, RecursiveMode::Recursive);
        }
        self.watcher = Some(watcher);
        Ok(())
    }

    pub fn stop(&mut self) {
        self.watcher = None;
    }

    pub fn introduced(&self) -> i64 {
        let guard = self.state.lock().expect("complexity state poisoned");
        i64::from(total(&guard.current)) - i64::from(total(&guard.baseline))
    }

    pub fn bytes_delta(&self) -> i64 {
        let guard = self.state.lock().expect("complexity state poisoned");
        bytes_delta(&guard.baseline_bytes, &guard.current_bytes)
    }

    pub fn files_delta(&self) -> i64 {
        let guard = self.state.lock().expect("complexity state poisoned");
        files_delta(&guard.baseline_bytes, &guard.current_bytes)
    }

    pub fn deltas(&self) -> Vec<ComplexityDelta> {
        let guard = self.state.lock().expect("complexity state poisoned");
        compute_deltas(&guard.baseline, &guard.current)
    }
}
