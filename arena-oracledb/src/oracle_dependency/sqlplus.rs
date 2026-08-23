const SCRIPT_PREAMBLE: &str = "WHENEVER SQLERROR EXIT SQL.SQLCODE ROLLBACK\nWHENEVER OSERROR EXIT 1\nSET DEFINE OFF\nSET PAGESIZE 0\nSET FEEDBACK OFF\nSET HEADING OFF\nSET ECHO OFF\nSET VERIFY OFF\nSET TRIMSPOOL ON\n";

pub const SQLPLUS_USER_ENV: &str = "SQLPLUS_USER";
pub const SQLPLUS_PASS_ENV: &str = "SQLPLUS_PASS";
const HEREDOC_DELIMITER: &str = "ARENA_ORACLE_SQLPLUS_EOF";

pub fn build_script(sql: &str) -> String {
    let mut script = String::from(SCRIPT_PREAMBLE);
    script.push_str(sql.trim_end());
    script.push('\n');
    script.push_str("EXIT;\n");
    script
}

pub fn build_exec_command(connect_target: &str, script: &str) -> Vec<String> {
    let shell = format!(
        "sqlplus -s \"${user}/${pass}@{connect_target}\" <<'{delim}'\n{script}{delim}\n",
        user = SQLPLUS_USER_ENV,
        pass = SQLPLUS_PASS_ENV,
        delim = HEREDOC_DELIMITER,
    );
    vec!["sh".to_string(), "-c".to_string(), shell]
}

pub fn parse_scalar_i32(stdout: &str) -> Result<i32, String> {
    let value = stdout
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .ok_or_else(|| format!("sqlplus scalar query returned no output: {stdout:?}"))?;

    if value.starts_with("ORA-") || value.starts_with("SP2-") {
        return Err(format!("sqlplus reported an error: {value}"));
    }

    value
        .parse::<i32>()
        .map_err(|e| format!("sqlplus scalar query output {value:?} is not an i32: {e}"))
}

pub fn parse_table_list(stdout: &str) -> Vec<String> {
    stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

pub fn parse_constraint_list(stdout: &str) -> Vec<(String, String)> {
    stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter_map(|line| {
            let (table, constraint) = line.split_once('|')?;
            let table = table.trim();
            let constraint = constraint.trim();
            if table.is_empty() || constraint.is_empty() {
                return None;
            }
            Some((table.to_string(), constraint.to_string()))
        })
        .collect()
}

pub fn is_success(exit_code: Option<i64>) -> bool {
    exit_code == Some(0)
}
