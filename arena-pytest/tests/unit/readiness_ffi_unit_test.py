from arena_pytest.ffi._ffi_readiness import readiness_checks_for_ffi
from arena_pytest.readiness import HttpReadinessCheck, TcpReadinessCheck


def test_http_check_serializes_kind_http():
    out = readiness_checks_for_ffi(
        [(HttpReadinessCheck.create(), "http://127.0.0.1:8080/health")]
    )
    assert out == [
        {"kind": "http", "target": "http://127.0.0.1:8080/health", "timeout_ms": 10_000}
    ]


def test_tcp_check_serializes_kind_tcp():
    out = readiness_checks_for_ffi([(TcpReadinessCheck.create(), "127.0.0.1:2525", 5_000)])
    assert out == [{"kind": "tcp", "target": "127.0.0.1:2525", "timeout_ms": 5_000}]
