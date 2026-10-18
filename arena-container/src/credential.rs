use crate::identifier::new_suffix;

const COMPLEXITY: &str = "Aa1!";

pub fn new_password(prefix: &str) -> String {
    format!("{prefix}{}{COMPLEXITY}", new_suffix())
}
