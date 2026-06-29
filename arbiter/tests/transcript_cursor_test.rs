mod harness_factory;

use arbiter::cursor::parse_line;
use arbiter::transcript::Role;
use harness_factory::HarnessFactory;

#[test]
fn parse_line_user_text_returns_prompt_chars() {
    let factory = HarnessFactory::cursor();
    let event = parse_line(&factory.user_line("hello")).unwrap();
    assert_eq!(event.role(), Role::User);
    assert_eq!(event.prompt_chars, 5);
}

#[test]
fn parse_line_assistant_write_returns_edit_bytes() {
    let factory = HarnessFactory::cursor();
    let event = parse_line(&factory.tool_call_line("Write", r#"{"contents":"abcd"}"#)).unwrap();
    assert_eq!(event.role(), Role::Assistant);
    assert_eq!(event.edit_bytes, 4);
}

#[test]
fn parse_line_strreplace_returns_new_string_len() {
    let factory = HarnessFactory::cursor();
    let event = parse_line(
        &factory.tool_call_line("StrReplace", r#"{"old_string":"x","new_string":"abcdef"}"#),
    )
    .unwrap();
    assert_eq!(event.edit_bytes, 6);
}

#[test]
fn parse_line_read_and_shell_increment_ops() {
    let factory = HarnessFactory::cursor();
    let read = parse_line(&factory.tool_call_line("Grep", r#"{"pattern":"x"}"#)).unwrap();
    let shell = parse_line(&factory.tool_call_line("Shell", r#"{"command":"ls"}"#)).unwrap();
    assert_eq!(read.read_ops, 1);
    assert_eq!(shell.shell_ops, 1);
}

#[test]
fn parse_line_string_content_returns_event() {
    let line = r#"{"role":"user","message":{"content":"why not do it differently"}}"#;
    let event = parse_line(line).unwrap();
    assert_eq!(event.role(), Role::User);
    assert!(event.probe_hits >= 2);
}

#[test]
fn parse_line_invalid_json_returns_none() {
    assert!(parse_line("not json").is_none());
}
