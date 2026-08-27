import json
import time


def claims_with_scope(issuer: str, scope: str) -> str:
    now = int(time.time())
    return json.dumps(
        {
            "iss": issuer,
            "sub": "arena-examples",
            "scope": scope,
            "iat": now,
            "exp": now + 300,
        }
    )
