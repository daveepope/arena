#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    User,
    Assistant,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolKind {
    Edit,
    Read,
    Shell,
    Other,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Event {
    pub role_user: bool,
    pub prompt_chars: usize,
    pub probe_hits: usize,
    pub edit_bytes: usize,
    pub read_ops: usize,
    pub shell_ops: usize,
}

impl Event {
    pub fn user() -> Self {
        Event {
            role_user: true,
            ..Event::default()
        }
    }

    pub fn assistant() -> Self {
        Event {
            role_user: false,
            ..Event::default()
        }
    }

    pub fn role(&self) -> Role {
        if self.role_user {
            Role::User
        } else {
            Role::Assistant
        }
    }
}

pub fn record_user_text(event: &mut Event, text: &str) {
    event.prompt_chars += text.chars().count();
    event.probe_hits += count_probes(text);
}

pub fn record_tool(event: &mut Event, kind: ToolKind, edit_bytes: usize) {
    match kind {
        ToolKind::Edit => event.edit_bytes += edit_bytes,
        ToolKind::Read => event.read_ops += 1,
        ToolKind::Shell => event.shell_ops += 1,
        ToolKind::Other => {}
    }
}

pub const PROBE_PHRASES: &[&str] = &[
    "why",
    "why did",
    "why are",
    "why is",
    "why does",
    "why not",
    "why would",
    "explain",
    "explanation",
    "can you explain",
    "could you explain",
    "what does",
    "what is",
    "what's the",
    "whats the",
    "what are",
    "what happens",
    "what happens if",
    "what if",
    "what about",
    "how does",
    "how do",
    "how is",
    "how would",
    "how come",
    "walk me through",
    "talk me through",
    "break it down",
    "break down",
    "clarify",
    "can you clarify",
    "i don't understand",
    "i dont understand",
    "not sure i understand",
    "help me understand",
    "make me understand",
    "are you sure",
    "is that correct",
    "is this correct",
    "is that right",
    "is this right",
    "does this work",
    "will this work",
    "did you test",
    "have you tested",
    "is it safe",
    "is this safe",
    "double check",
    "double-check",
    "verify",
    "confirm",
    "make sure",
    "prove it",
    "show me",
    "show me where",
    "that's wrong",
    "thats wrong",
    "that is wrong",
    "this is wrong",
    "actually",
    "instead",
    "revert",
    "undo",
    "roll back",
    "rollback",
    "don't",
    "do not",
    "rather than",
    "shouldn't",
    "should not",
    "incorrect",
    "that's a bug",
    "is a bug",
    "broken",
    "doesn't work",
    "does not work",
    "didn't work",
    "alternative",
    "alternatives",
    "trade-off",
    "tradeoff",
    "trade off",
    "trade-offs",
    "pros and cons",
    "downside",
    "downsides",
    "better way",
    "any risk",
    "any risks",
    "edge case",
    "edge-case",
    "edge cases",
    "corner case",
    "consider",
    "could we",
    "should we",
    "implications",
];

pub fn count_probes(text: &str) -> usize {
    let lowered = text.to_lowercase();
    PROBE_PHRASES
        .iter()
        .filter(|phrase| lowered.contains(*phrase))
        .count()
}
