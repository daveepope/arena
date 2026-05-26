import ctypes
import json
import os
from typing import Any, Dict
from urllib.parse import urlparse

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

    def build(self) -> "OauthDependency":
        cfg = dict(self._config)
        if not str(cfg.get("metadata_base_url") or "").strip():
            cfg["metadata_base_url"] = OAUTH_ISSUER
        return OauthDependency(cfg)


class OauthDependency:
    def __init__(self, config: Dict[str, Any]):
        self._config = config

    @property
    def identifier(self) -> str:
        return str(self._config["identifier"])

    def _for_ffi(self) -> Dict[str, Any]:
        return dict(self._config)


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
