pub fn log_line(identifier: &str, line: &str) {
    if line.contains(" ERROR ") {
        tracing::error!(component = %identifier, "{}", line);
    } else if line.contains(" WARN ") {
        tracing::warn!(component = %identifier, "{}", line);
    } else if line.contains(" DEBUG ") {
        tracing::debug!(component = %identifier, "{}", line);
    } else if line.contains(" TRACE ") {
        tracing::trace!(component = %identifier, "{}", line);
    } else {
        tracing::debug!(component = %identifier, "{}", line);
    }
}
