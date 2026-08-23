use arena_oracledb::oracle_dependency::sqlplus;

#[test]
fn build_script_plain_statement_wraps_with_error_handling_preamble() {
    let script = sqlplus::build_script("SELECT 1 FROM dual");

    assert!(script.starts_with("WHENEVER SQLERROR EXIT SQL.SQLCODE ROLLBACK\n"));
    assert!(script.contains("SELECT 1 FROM dual\n"));
    assert!(script.ends_with("EXIT;\n"));
}

#[test]
fn build_script_trailing_whitespace_is_trimmed_before_exit() {
    let script = sqlplus::build_script("SELECT 1 FROM dual;   \n\n");

    assert!(script.contains("SELECT 1 FROM dual;\nEXIT;\n"));
}

#[test]
fn build_exec_command_wraps_script_in_quoted_heredoc() {
    let cmd = sqlplus::build_exec_command("//localhost:1521/FREEPDB1", "SELECT 1;\nEXIT;\n");

    assert_eq!(cmd.len(), 3);
    assert_eq!(cmd[0], "sh");
    assert_eq!(cmd[1], "-c");
    assert!(cmd[2].contains("sqlplus -s \"$SQLPLUS_USER/$SQLPLUS_PASS@//localhost:1521/FREEPDB1\""));
    assert!(cmd[2].contains("<<'ARENA_ORACLE_SQLPLUS_EOF'"));
    assert!(cmd[2].contains("SELECT 1;\nEXIT;\nARENA_ORACLE_SQLPLUS_EOF"));
}

#[test]
fn build_exec_command_script_containing_bare_eof_line_does_not_end_heredoc_early() {
    let cmd = sqlplus::build_exec_command("//localhost:1521/FREEPDB1", "SELECT 'EOF' FROM dual;\nEXIT;\n");

    assert!(cmd[2].contains("SELECT 'EOF' FROM dual;\nEXIT;\nARENA_ORACLE_SQLPLUS_EOF"));
}

#[test]
fn build_exec_command_sql_with_dollar_and_quotes_is_not_interpolated() {
    let sql = sqlplus::build_script("INSERT INTO widgets (name) VALUES ('it''s $5');");
    let cmd = sqlplus::build_exec_command("//localhost:1521/FREEPDB1", &sql);

    assert!(cmd[2].contains("'it''s $5'"));
}

#[test]
fn parse_scalar_i32_single_value_returns_value() {
    let result = sqlplus::parse_scalar_i32("  42  \n");

    assert_eq!(result, Ok(42));
}

#[test]
fn parse_scalar_i32_blank_lines_around_value_skips_blanks() {
    let result = sqlplus::parse_scalar_i32("\n\n   7\n\n");

    assert_eq!(result, Ok(7));
}

#[test]
fn parse_scalar_i32_empty_output_returns_err() {
    let result = sqlplus::parse_scalar_i32("   \n  \n");

    assert!(result.is_err());
}

#[test]
fn parse_scalar_i32_non_numeric_output_returns_err() {
    let result = sqlplus::parse_scalar_i32("not-a-number");

    assert!(result.is_err());
}

#[test]
fn parse_scalar_i32_ora_error_banner_returns_clear_error() {
    let result = sqlplus::parse_scalar_i32("ORA-00942: table or view does not exist");

    assert_eq!(
        result,
        Err("sqlplus reported an error: ORA-00942: table or view does not exist".to_string())
    );
}

#[test]
fn parse_scalar_i32_sp2_error_banner_returns_clear_error() {
    let result = sqlplus::parse_scalar_i32("SP2-0734: unknown command beginning \"SELEC...\"");

    assert!(result.unwrap_err().starts_with("sqlplus reported an error:"));
}

#[test]
fn parse_table_list_multiple_lines_returns_trimmed_names() {
    let tables = sqlplus::parse_table_list("  WIDGETS  \nGADGETS\n\n");

    assert_eq!(tables, vec!["WIDGETS".to_string(), "GADGETS".to_string()]);
}

#[test]
fn parse_table_list_empty_output_returns_empty_vec() {
    let tables = sqlplus::parse_table_list("\n\n   \n");

    assert!(tables.is_empty());
}

#[test]
fn parse_constraint_list_pipe_delimited_lines_returns_pairs() {
    let pairs = sqlplus::parse_constraint_list("WIDGETS|FK_WIDGETS_GADGET\nGADGETS|FK_GADGETS_OWNER\n");

    assert_eq!(
        pairs,
        vec![
            ("WIDGETS".to_string(), "FK_WIDGETS_GADGET".to_string()),
            ("GADGETS".to_string(), "FK_GADGETS_OWNER".to_string()),
        ]
    );
}

#[test]
fn parse_constraint_list_line_without_pipe_is_skipped() {
    let pairs = sqlplus::parse_constraint_list("WIDGETS_NO_DELIMITER\nGADGETS|FK_GADGETS_OWNER\n");

    assert_eq!(pairs, vec![("GADGETS".to_string(), "FK_GADGETS_OWNER".to_string())]);
}

#[test]
fn is_success_zero_exit_code_returns_true() {
    assert!(sqlplus::is_success(Some(0)));
}

#[test]
fn is_success_nonzero_exit_code_returns_false() {
    assert!(!sqlplus::is_success(Some(1)));
}

#[test]
fn is_success_missing_exit_code_returns_false() {
    assert!(!sqlplus::is_success(None));
}
