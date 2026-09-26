import ctypes

import pytest

from arena_pytest.ffi._ffi import find_lib, load_ffi

ARENA_NATIVE_EXPORTS = (
    "arena_active_playbook_drop",
    "arena_add_lifecycle_observer",
    "arena_add_log_target",
    "arena_close",
    "arena_dispatcher_component_allow_json_set",
    "arena_dispatcher_default_logging_target_logger_name_utf8",
    "arena_dispatcher_default_logging_target_publish_level",
    "arena_dispatcher_dependency_allow_json_set",
    "arena_find_available_port",
    "arena_free_string",
    "arena_hard_reset",
    "arena_http_playbook_open",
    "arena_http_playbook_verify",
    "arena_match_playbook_run",
    "arena_mssql_playbook_verify",
    "arena_oauth_loopback_tls_pem_json",
    "arena_oauth_sign_claims",
    "arena_open",
    "arena_oracle_playbook_verify",
    "arena_postgres_playbook_verify",
    "arena_remove_lifecycle_observer",
    "arena_remove_log_target",
    "arena_set_log_level",
    "arena_soft_reset",
    "arena_state_json",
)


def _unconfigured_native():
    path = find_lib()
    assert path, "arena shared library must be resolvable for this test"
    return ctypes.CDLL(path)


@pytest.mark.parametrize("symbol", ARENA_NATIVE_EXPORTS)
def test_native_declared_export_resolves(symbol):
    native = _unconfigured_native()

    assert getattr(native, symbol) is not None


def test_load_ffi_configures_every_symbol_the_client_calls():
    ffi = load_ffi()
    assert ffi is not None

    configured = {
        name for name, value in vars(ffi.lib).items() if isinstance(value, ctypes._CFuncPtr)
    }

    assert configured
    assert configured <= set(ARENA_NATIVE_EXPORTS), (
        "the client configured a symbol the native does not export: "
        f"{sorted(configured - set(ARENA_NATIVE_EXPORTS))}"
    )
