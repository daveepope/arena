use chrono::{DateTime, SecondsFormat, Utc};
use serde::Serialize;
use std::fmt;

use super::fault::{serialize_timestamp, Fault};
use super::state::{ArenaLifecycleState, RunnableState};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DependencyState {
    pub id: String,
    pub state: RunnableState,
    pub faults: Vec<Fault>,
    pub children: Vec<DependencyState>,
}

impl DependencyState {
    pub fn new(
        id: impl Into<String>,
        state: RunnableState,
        faults: Vec<Fault>,
        children: Vec<DependencyState>,
    ) -> Self {
        Self {
            id: id.into(),
            state,
            faults,
            children,
        }
    }

    pub fn has_faulted(&self) -> bool {
        self.state == RunnableState::Faulted || self.children.iter().any(|c| c.has_faulted())
    }

    pub fn collect_faults<'a>(&'a self, out: &mut Vec<&'a Fault>) {
        out.extend(self.faults.iter());
        for child in &self.children {
            child.collect_faults(out);
        }
    }

    pub fn find(&self, id: &str) -> Option<&DependencyState> {
        if self.id == id {
            return Some(self);
        }
        self.children.iter().find_map(|c| c.find(id))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ComponentState {
    pub id: String,
    pub state: RunnableState,
    pub faults: Vec<Fault>,
    pub children: Vec<ComponentState>,
}

impl ComponentState {
    pub fn new(
        id: impl Into<String>,
        state: RunnableState,
        faults: Vec<Fault>,
        children: Vec<ComponentState>,
    ) -> Self {
        Self {
            id: id.into(),
            state,
            faults,
            children,
        }
    }

    pub fn has_faulted(&self) -> bool {
        self.state == RunnableState::Faulted || self.children.iter().any(|c| c.has_faulted())
    }

    pub fn collect_faults<'a>(&'a self, out: &mut Vec<&'a Fault>) {
        out.extend(self.faults.iter());
        for child in &self.children {
            child.collect_faults(out);
        }
    }

    pub fn find(&self, id: &str) -> Option<&ComponentState> {
        if self.id == id {
            return Some(self);
        }
        self.children.iter().find_map(|c| c.find(id))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ArenaState {
    pub id: String,
    pub state: ArenaLifecycleState,
    #[serde(serialize_with = "serialize_timestamp")]
    pub at: DateTime<Utc>,
    pub dependencies: Vec<DependencyState>,
    pub components: Vec<ComponentState>,
    pub faults: Vec<Fault>,
}

impl ArenaState {
    pub fn new(
        id: impl Into<String>,
        state: ArenaLifecycleState,
        dependencies: Vec<DependencyState>,
        components: Vec<ComponentState>,
        arena_faults: Vec<Fault>,
    ) -> Self {
        let faults = aggregate_faults(&dependencies, &components, arena_faults);
        Self {
            id: id.into(),
            state,
            at: Utc::now(),
            dependencies,
            components,
            faults,
        }
    }

    pub fn has_faulted_subject(&self) -> bool {
        self.dependencies.iter().any(|d| d.has_faulted())
            || self.components.iter().any(|c| c.has_faulted())
    }

    pub fn terminal_state(&self) -> ArenaLifecycleState {
        if self.has_faulted_subject() || !self.faults.is_empty() {
            ArenaLifecycleState::ArenaFaulted
        } else {
            ArenaLifecycleState::ArenaClosed
        }
    }

    pub fn dependency(&self, id: &str) -> Option<&DependencyState> {
        self.dependencies.iter().find_map(|d| d.find(id))
    }

    pub fn component(&self, id: &str) -> Option<&ComponentState> {
        self.components.iter().find_map(|c| c.find(id))
    }

    pub fn timestamp(&self) -> String {
        self.at.to_rfc3339_opts(SecondsFormat::Millis, true)
    }
}

pub fn aggregate_faults(
    dependencies: &[DependencyState],
    components: &[ComponentState],
    arena_faults: Vec<Fault>,
) -> Vec<Fault> {
    let mut borrowed = Vec::new();
    for dependency in dependencies {
        dependency.collect_faults(&mut borrowed);
    }
    for component in components {
        component.collect_faults(&mut borrowed);
    }
    let mut faults: Vec<Fault> = borrowed.into_iter().cloned().collect();
    faults.extend(arena_faults);
    faults.sort_by_key(|f| f.at);
    let mut unique: Vec<Fault> = Vec::with_capacity(faults.len());
    for fault in faults {
        if !unique.contains(&fault) {
            unique.push(fault);
        }
    }
    unique
}

impl DependencyState {
    fn render(&self, f: &mut fmt::Formatter<'_>, depth: usize) -> fmt::Result {
        let indent = "  ".repeat(depth);
        write!(f, "\n{indent}'{}': {}", self.id, self.state)?;
        for child in &self.children {
            child.render(f, depth + 1)?;
        }
        Ok(())
    }
}

impl ComponentState {
    fn render(&self, f: &mut fmt::Formatter<'_>, depth: usize) -> fmt::Result {
        let indent = "  ".repeat(depth);
        write!(f, "\n{indent}'{}': {}", self.id, self.state)?;
        for child in &self.children {
            child.render(f, depth + 1)?;
        }
        Ok(())
    }
}

impl fmt::Display for ArenaState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "arena '{}' is {} at {}",
            self.id,
            self.state,
            self.timestamp()
        )?;
        if !self.dependencies.is_empty() {
            write!(f, "\n  dependencies:")?;
            for dependency in &self.dependencies {
                dependency.render(f, 2)?;
            }
        }
        if !self.components.is_empty() {
            write!(f, "\n  components:")?;
            for component in &self.components {
                component.render(f, 2)?;
            }
        }
        if !self.faults.is_empty() {
            write!(f, "\n  faults:")?;
            for fault in &self.faults {
                write!(f, "\n    ")?;
                fault.render(f, 2)?;
            }
        }
        Ok(())
    }
}

impl std::error::Error for ArenaState {}
