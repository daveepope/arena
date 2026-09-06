use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ArenaLifecycleState {
    ArenaCreated,
    ArenaStarting,
    DependenciesStarting,
    DependenciesStarted,
    PlaybooksRunning,
    PlaybooksComplete,
    ComponentsStarting,
    ComponentsStarted,
    ArenaOpen,
    ArenaClosing,
    ComponentsStopping,
    ComponentsStopped,
    DependenciesStopping,
    DependenciesStopped,
    ArenaTeardown,
    ArenaClosed,
    ArenaFaulted,
}

impl ArenaLifecycleState {
    pub fn is_final(&self) -> bool {
        matches!(self, Self::ArenaClosed | Self::ArenaFaulted)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ArenaCreated => "arena_created",
            Self::ArenaStarting => "arena_starting",
            Self::DependenciesStarting => "dependencies_starting",
            Self::DependenciesStarted => "dependencies_started",
            Self::PlaybooksRunning => "playbooks_running",
            Self::PlaybooksComplete => "playbooks_complete",
            Self::ComponentsStarting => "components_starting",
            Self::ComponentsStarted => "components_started",
            Self::ArenaOpen => "arena_open",
            Self::ArenaClosing => "arena_closing",
            Self::ComponentsStopping => "components_stopping",
            Self::ComponentsStopped => "components_stopped",
            Self::DependenciesStopping => "dependencies_stopping",
            Self::DependenciesStopped => "dependencies_stopped",
            Self::ArenaTeardown => "arena_teardown",
            Self::ArenaClosed => "arena_closed",
            Self::ArenaFaulted => "arena_faulted",
        }
    }
}

impl fmt::Display for ArenaLifecycleState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RunnableState {
    NotStarted,
    Starting,
    ReadinessCheck,
    Started,
    Stopping,
    Stopped,
    Faulted,
}

impl RunnableState {
    pub fn is_final(&self) -> bool {
        matches!(self, Self::Stopped | Self::Faulted)
    }

    pub fn is_inactive(&self) -> bool {
        matches!(self, Self::NotStarted | Self::Stopped)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NotStarted => "not_started",
            Self::Starting => "starting",
            Self::ReadinessCheck => "readiness_check",
            Self::Started => "started",
            Self::Stopping => "stopping",
            Self::Stopped => "stopped",
            Self::Faulted => "faulted",
        }
    }
}

impl Default for RunnableState {
    fn default() -> Self {
        Self::NotStarted
    }
}

impl fmt::Display for RunnableState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
