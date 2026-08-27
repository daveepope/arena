import ctypes
import json
import os
from typing import Any, Dict, List, Optional
from urllib.parse import urlparse

from arena_pytest.ffi._ffi_children import children_for_ffi
from arena_pytest.support._identifier import build as _build_identifier

DEFAULT_OAUTH_PORT = 9444
_oauth_env = os.environ.get("ARENA_PYTEST_OAUTH_ISSUER", "").strip().rstrip("/")
OAUTH_ISSUER = _oauth_env or f"https://127.0.0.1:{DEFAULT_OAUTH_PORT}"
_oauth_host = (urlparse(OAUTH_ISSUER).hostname or "").lower()
oauth_issuer_host_is_non_loopback = bool(_oauth_host) and _oauth_host not in (
    "127.0.0.1",
    "localhost",
    "::1",
)


class OauthDependencyBuilder:
    def __init__(self, name: str = ""):
        self._config: Dict[str, Any] = {
            "type": "oauth",
            "identifier": _build_identifier("arena-oauth", name),
            "port": DEFAULT_OAUTH_PORT,
        }
        self._children: List[Any] = []

    def with_port(self, port: int) -> "OauthDependencyBuilder":
        self._config["port"] = port
        return self

    def with_listen_ip(self, ip: str) -> "OauthDependencyBuilder":
        self._config["listen_ip"] = ip
        return self

    def with_server_tls_pem(self, cert_pem: str, key_pem: str) -> "OauthDependencyBuilder":
        self._config["server_tls_certificate_pem"] = cert_pem
        self._config["server_tls_private_key_pem"] = key_pem
        self._config["transport"] = "tls"
        return self

    def with_http(self) -> "OauthDependencyBuilder":
        self._config["transport"] = "http"
        return self

    def with_metadata_base_url(self, url: str) -> "OauthDependencyBuilder":
        self._config["metadata_base_url"] = url.rstrip("/")
        return self

    def with_child_dependencies(self, children: List[Any]) -> "OauthDependencyBuilder":
        self._children.extend(children)
        return self

    def with_issuer_cognito(self, pool_id: str) -> "OauthDependencyBuilder":
        self._config.setdefault("issuers", []).append(
            {"provider": "cognito", "pool_id": pool_id}
        )
        return self

    def with_issuer_okta(self) -> "OauthDependencyBuilder":
        self._config.setdefault("issuers", []).append({"provider": "okta"})
        return self

    def with_issuer_entra_id(self, tenant_id: str) -> "OauthDependencyBuilder":
        self._config.setdefault("issuers", []).append(
            {"provider": "entra_id", "tenant_id": tenant_id}
        )
        return self

    def with_issuer(
        self,
        issuer_path: Optional[str] = None,
        jwks_path: Optional[str] = None,
        rsa_pkcs8_pem: Optional[str] = None,
    ) -> "OauthDependencyBuilder":
        entry: Dict[str, Any] = {"provider": "custom"}
        if issuer_path is not None:
            entry["issuer_path"] = issuer_path
        if jwks_path is not None:
            entry["jwks_path"] = jwks_path
        if rsa_pkcs8_pem is not None:
            entry["rsa_pkcs8_pem"] = rsa_pkcs8_pem
        self._config.setdefault("issuers", []).append(entry)
        return self

    def build(self) -> "OauthDependency":
        cfg = dict(self._config)
        if not str(cfg.get("metadata_base_url") or "").strip():
            cfg["metadata_base_url"] = OAUTH_ISSUER
        return OauthDependency(cfg, children=list(self._children))


class OauthDependency:
    def __init__(self, config: Dict[str, Any], children: Optional[List[Any]] = None):
        self._config = config
        self._children = children or []

    @property
    def identifier(self) -> str:
        return str(self._config["identifier"])

    def _for_ffi(self) -> Dict[str, Any]:
        d = dict(self._config)
        children = children_for_ffi(self._children)
        if children:
            d["children"] = children
        return d


def oauth_loopback_tls_pem_pair() -> tuple[str, str]:
    from arena_pytest.ffi._ffi import ArenaBindingError, load_ffi, _take_err

    ffi = load_ffi()
    if ffi is None:
        raise RuntimeError(
            "arena_ffi shared library not found (required for oauth_loopback_tls_pem_pair)"
        )
    err = ctypes.c_void_p()
    raw = ffi.lib.arena_oauth_loopback_tls_pem_json(ctypes.byref(err))
    if not raw:
        msg = _take_err(err, ffi) or "arena_oauth_loopback_tls_pem_json returned null"
        raise ArenaBindingError(msg)
    try:
        payload = json.loads(ctypes.string_at(raw).decode("utf-8"))
    finally:
        ffi.lib.arena_free_string(raw)
    cert = payload.get("certificate_pem")
    key = payload.get("private_key_pem")
    if not isinstance(cert, str) or not isinstance(key, str):
        raise ArenaBindingError(
            "arena_oauth_loopback_tls_pem_json: missing certificate_pem or private_key_pem"
        )
    return cert, key
