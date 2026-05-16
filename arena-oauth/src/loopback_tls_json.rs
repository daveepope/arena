use serde_json::json;

use crate::ephemeral_tls;

pub fn loopback_tls_pem_json_document() -> Result<String, String> {
    let (certificate_pem, private_key_pem) = ephemeral_tls::localhost_self_signed_pem_pair()?;
    serde_json::to_string(&json!({
        "certificate_pem": certificate_pem,
        "private_key_pem": private_key_pem,
    }))
    .map_err(|e| format!("loopback tls pem json encode: {e}"))
}
