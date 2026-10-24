use crate::identifier::new_suffix;

const PREFIX_LENGTH: usize = 3;

pub fn new_password() -> String {
    let suffix = new_suffix();
    let mut out = String::with_capacity(PREFIX_LENGTH + 2 * suffix.len());
    let seed = suffix
        .bytes()
        .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));

    out.push(char::from(b'A' + (seed % 26) as u8));
    out.push(char::from(b'a' + ((seed / 26) % 26) as u8));
    out.push(char::from(b'0' + ((seed / 676) % 10) as u8));
    for b in suffix.bytes() {
        out.push(char::from(b.to_ascii_uppercase()));
        out.push(char::from(b));
    }
    out
}
