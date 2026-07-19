import contextlib
import io
import json
import os
import socket
import subprocess
import sys
import tempfile
import unittest
import urllib.error
from pathlib import Path
from unittest.mock import MagicMock, patch

sys.path.insert(0, str(Path(__file__).resolve().parent))

from check_container_cves import (
    ScanError,
    _free_port,
    _trivy_canonical_repos,
    build_row,
    distinct_vulnerabilities,
    find_trivy_bin,
    main,
    render_table,
    scan_all_images,
    scan_error_entries,
    scan_error_reason,
    scan_image,
    severities_from_env,
    show_vulnerability_ids_from_env,
    start_trivy_server,
    stop_trivy_server,
)

_ENTRIES = [
    {"id": "postgres", "image": "postgres", "tag": "17"},
    {"id": "mssql", "image": "mcr.microsoft.com/mssql/server", "tag": "2022-CU25-ubuntu-22.04"},
]

_EMPTY_RESULT = {"Results": [{"Vulnerabilities": []}]}


class BuildRowTest(unittest.TestCase):
    def test_build_row_no_vulnerabilities_returns_zero_counts(self) -> None:
        row = build_row(_ENTRIES[0], _EMPTY_RESULT)

        self.assertEqual(
            row,
            {
                "ID": "postgres",
                "IMAGE": "postgres:17",
                "DISTINCT CRITICAL CVEs": "0",
                "DISTINCT HIGH CVEs": "0",
            },
        )

    def test_build_row_missing_results_key_returns_zero_counts(self) -> None:
        row = build_row(_ENTRIES[0], {})

        self.assertEqual(row["DISTINCT CRITICAL CVEs"], "0")
        self.assertEqual(row["DISTINCT HIGH CVEs"], "0")

    def test_build_row_vulnerabilities_present_reports_severity_counts(self) -> None:
        raw = {
            "Results": [
                {
                    "Vulnerabilities": [
                        {"VulnerabilityID": "CVE-2026-1234", "Severity": "CRITICAL"},
                        {"VulnerabilityID": "CVE-2026-1235", "Severity": "CRITICAL"},
                        {"VulnerabilityID": "CVE-2026-1236", "Severity": "HIGH"},
                    ]
                }
            ]
        }

        row = build_row(_ENTRIES[0], raw)

        self.assertEqual(
            row,
            {
                "ID": "postgres",
                "IMAGE": "postgres:17",
                "DISTINCT CRITICAL CVEs": "2",
                "DISTINCT HIGH CVEs": "1",
            },
        )

    def test_build_row_same_cve_across_multiple_packages_counts_once(self) -> None:
        raw = {
            "Results": [
                {
                    "Vulnerabilities": [
                        {
                            "VulnerabilityID": "CVE-2026-1234",
                            "Severity": "CRITICAL",
                            "PkgName": "libfoo",
                        },
                        {
                            "VulnerabilityID": "CVE-2026-1234",
                            "Severity": "CRITICAL",
                            "PkgName": "libbar",
                        },
                    ]
                }
            ]
        }

        row = build_row(_ENTRIES[0], raw)

        self.assertEqual(row["DISTINCT CRITICAL CVEs"], "1")


class DistinctVulnerabilitiesTest(unittest.TestCase):
    def test_distinct_vulnerabilities_no_vulnerabilities_returns_empty(self) -> None:
        self.assertEqual(distinct_vulnerabilities(_EMPTY_RESULT), {})

    def test_distinct_vulnerabilities_missing_results_key_returns_empty(self) -> None:
        self.assertEqual(distinct_vulnerabilities({}), {})

    def test_distinct_vulnerabilities_tracks_severity_and_package(self) -> None:
        raw = {
            "Results": [
                {
                    "Vulnerabilities": [
                        {
                            "VulnerabilityID": "CVE-2026-1234",
                            "Severity": "CRITICAL",
                            "PkgName": "libfoo",
                        }
                    ]
                }
            ]
        }

        vulns = distinct_vulnerabilities(raw)

        self.assertEqual(set(vulns), {"CVE-2026-1234"})
        self.assertEqual(vulns["CVE-2026-1234"].severity, "CRITICAL")
        self.assertEqual(vulns["CVE-2026-1234"].packages, ["libfoo"])

    def test_distinct_vulnerabilities_same_cve_multiple_packages_merges_package_list(self) -> None:
        raw = {
            "Results": [
                {
                    "Vulnerabilities": [
                        {
                            "VulnerabilityID": "CVE-2026-1234",
                            "Severity": "CRITICAL",
                            "PkgName": "libfoo",
                        },
                        {
                            "VulnerabilityID": "CVE-2026-1234",
                            "Severity": "CRITICAL",
                            "PkgName": "libbar",
                        },
                    ]
                }
            ]
        }

        vulns = distinct_vulnerabilities(raw)

        self.assertEqual(len(vulns), 1)
        self.assertEqual(vulns["CVE-2026-1234"].packages, ["libfoo", "libbar"])

    def test_distinct_vulnerabilities_same_cve_same_package_deduplicates_package(self) -> None:
        raw = {
            "Results": [
                {
                    "Vulnerabilities": [
                        {
                            "VulnerabilityID": "CVE-2026-1234",
                            "Severity": "CRITICAL",
                            "PkgName": "libfoo",
                        },
                        {
                            "VulnerabilityID": "CVE-2026-1234",
                            "Severity": "CRITICAL",
                            "PkgName": "libfoo",
                        },
                    ]
                }
            ]
        }

        vulns = distinct_vulnerabilities(raw)

        self.assertEqual(vulns["CVE-2026-1234"].packages, ["libfoo"])

    def test_distinct_vulnerabilities_spans_multiple_results(self) -> None:
        raw = {
            "Results": [
                {"Vulnerabilities": [{"VulnerabilityID": "CVE-2026-0001", "Severity": "HIGH"}]},
                {"Vulnerabilities": [{"VulnerabilityID": "CVE-2026-0002", "Severity": "CRITICAL"}]},
            ]
        }

        vulns = distinct_vulnerabilities(raw)

        self.assertEqual(set(vulns), {"CVE-2026-0001", "CVE-2026-0002"})


class RenderTableTest(unittest.TestCase):
    def test_render_table_borders_header_and_rows_with_box_drawing_characters(self) -> None:
        rows = [
            {
                "ID": "postgres",
                "IMAGE": "postgres:17",
                "DISTINCT CRITICAL CVEs": "2",
                "DISTINCT HIGH CVEs": "1",
            },
            {
                "ID": "http",
                "IMAGE": "wiremock/wiremock:3.13.2-alpine",
                "DISTINCT CRITICAL CVEs": "0",
                "DISTINCT HIGH CVEs": "0",
            },
        ]

        table = render_table(rows)
        lines = table.splitlines()

        self.assertEqual(len(lines), 6)
        self.assertTrue(lines[0].startswith("┌") and lines[0].endswith("┐"))
        self.assertIn("ID", lines[1])
        self.assertIn("DISTINCT CRITICAL CVEs", lines[1])
        self.assertIn("DISTINCT HIGH CVEs", lines[1])
        self.assertTrue(lines[2].startswith("├") and lines[2].endswith("┤"))
        self.assertIn("postgres", lines[3])
        self.assertIn("wiremock/wiremock:3.13.2-alpine", lines[4])
        self.assertTrue(lines[5].startswith("└") and lines[5].endswith("┘"))
        for line in (lines[3], lines[4]):
            self.assertTrue(line.startswith("│") and line.endswith("│"))

    def test_render_table_no_rows_still_renders_header_and_borders(self) -> None:
        table = render_table([])
        lines = table.splitlines()

        self.assertEqual(len(lines), 4)
        self.assertIn("ID", lines[1])
        self.assertTrue(lines[3].startswith("└") and lines[3].endswith("┘"))


class ScanErrorEntriesTest(unittest.TestCase):
    def test_scan_error_entries_no_errors_returns_empty(self) -> None:
        scan_results = {"postgres": _EMPTY_RESULT, "mssql": _EMPTY_RESULT}

        self.assertEqual(scan_error_entries(_ENTRIES, scan_results), [])

    def test_scan_error_entries_includes_rate_limited_and_non_rate_limited(self) -> None:
        rate_limited_error = ScanError("rate limited", rate_limited=True)
        other_error = ScanError("connection reset")
        scan_results = {"postgres": rate_limited_error, "mssql": other_error}

        errors = scan_error_entries(_ENTRIES, scan_results)

        self.assertEqual(
            errors,
            [(_ENTRIES[0], rate_limited_error), (_ENTRIES[1], other_error)],
        )

    def test_scan_error_entries_mixed_success_and_error_only_includes_the_error(self) -> None:
        error = ScanError("connection reset")
        scan_results = {"postgres": _EMPTY_RESULT, "mssql": error}

        errors = scan_error_entries(_ENTRIES, scan_results)

        self.assertEqual(errors, [(_ENTRIES[1], error)])


class ScanErrorReasonTest(unittest.TestCase):
    def test_scan_error_reason_rate_limited_mentions_rate_limit(self) -> None:
        reason = scan_error_reason(ScanError("rate limited", rate_limited=True))

        self.assertIn("rate limited", reason)

    def test_scan_error_reason_non_rate_limited_returns_original_message(self) -> None:
        reason = scan_error_reason(ScanError("connection reset"))

        self.assertEqual(reason, "connection reset")


class SeveritiesFromEnvTest(unittest.TestCase):
    def test_severities_from_env_unset_returns_default(self) -> None:
        with patch.dict(os.environ, {}, clear=False):
            os.environ.pop("ARENA_DEFAULT_CONTAINER_CVE_SEVERITIES", None)

            self.assertEqual(severities_from_env(), "CRITICAL,HIGH")

    def test_severities_from_env_set_returns_override(self) -> None:
        with patch.dict(os.environ, {"ARENA_DEFAULT_CONTAINER_CVE_SEVERITIES": "CRITICAL"}):
            self.assertEqual(severities_from_env(), "CRITICAL")

    def test_severities_from_env_blank_returns_default(self) -> None:
        with patch.dict(os.environ, {"ARENA_DEFAULT_CONTAINER_CVE_SEVERITIES": "   "}):
            self.assertEqual(severities_from_env(), "CRITICAL,HIGH")


class ShowVulnerabilityIdsFromEnvTest(unittest.TestCase):
    def test_show_vulnerability_ids_from_env_unset_returns_false(self) -> None:
        with patch.dict(os.environ, {}, clear=False):
            os.environ.pop("ARENA_CONTAINER_CVE_SHOW_IDS", None)

            self.assertFalse(show_vulnerability_ids_from_env())

    def test_show_vulnerability_ids_from_env_true_returns_true(self) -> None:
        with patch.dict(os.environ, {"ARENA_CONTAINER_CVE_SHOW_IDS": "true"}):
            self.assertTrue(show_vulnerability_ids_from_env())

    def test_show_vulnerability_ids_from_env_one_returns_true(self) -> None:
        with patch.dict(os.environ, {"ARENA_CONTAINER_CVE_SHOW_IDS": "1"}):
            self.assertTrue(show_vulnerability_ids_from_env())

    def test_show_vulnerability_ids_from_env_other_value_returns_false(self) -> None:
        with patch.dict(os.environ, {"ARENA_CONTAINER_CVE_SHOW_IDS": "no"}):
            self.assertFalse(show_vulnerability_ids_from_env())


def _fake_run_result(payload: dict, returncode: int = 0, stderr: str = "") -> MagicMock:
    return MagicMock(returncode=returncode, stdout=json.dumps(payload), stderr=stderr)


def _fake_healthz_response() -> MagicMock:
    response = MagicMock()
    response.status = 200
    response.__enter__.return_value = response
    response.__exit__.return_value = False
    return response


class TrivyCanonicalReposTest(unittest.TestCase):
    def test_trivy_canonical_repos_no_trivy_data_dep_returns_empty(self) -> None:
        from bazel_tools.tools.python.runfiles import runfiles

        r = runfiles.Create()
        self.assertIsNotNone(r, "bazel test always provides runfiles")

        canonical_repos = _trivy_canonical_repos(r)

        self.assertEqual(canonical_repos, [])


class FindTrivyBinTest(unittest.TestCase):
    def test_find_trivy_bin_env_var_points_to_real_file_returns_it(self) -> None:
        with tempfile.NamedTemporaryFile() as f:
            with patch.dict(os.environ, {"ARENA_TRIVY_BIN": f.name}):
                self.assertEqual(find_trivy_bin(), f.name)

    def test_find_trivy_bin_not_found_raises(self) -> None:
        with patch.dict(os.environ, {}, clear=False):
            os.environ.pop("ARENA_TRIVY_BIN", None)

            with self.assertRaisesRegex(RuntimeError, "trivy binary not found"):
                find_trivy_bin()


class FreePortTest(unittest.TestCase):
    def test_free_port_returns_immediately_bindable_port(self) -> None:
        port = _free_port()

        self.assertIsInstance(port, int)
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
            s.bind(("127.0.0.1", port))


class ScanImageTest(unittest.TestCase):
    def test_scan_image_success_returns_parsed_json(self) -> None:
        with patch(
            "check_container_cves.subprocess.run", return_value=_fake_run_result(_EMPTY_RESULT)
        ) as run_mock:
            raw = scan_image("trivy", "http://127.0.0.1:1", "postgres:17", "CRITICAL,HIGH")

        self.assertEqual(raw, _EMPTY_RESULT)
        command = run_mock.call_args.args[0]
        self.assertEqual(command[0], "trivy")
        self.assertIn("postgres:17", command)
        self.assertIn("CRITICAL,HIGH", command)

    def test_scan_image_nonzero_exit_raises_with_stderr(self) -> None:
        with patch(
            "check_container_cves.subprocess.run",
            return_value=_fake_run_result({}, returncode=1, stderr="boom"),
        ):
            with self.assertRaisesRegex(RuntimeError, "boom") as ctx:
                scan_image("trivy", "http://127.0.0.1:1", "postgres:17", "CRITICAL,HIGH")

        self.assertIsInstance(ctx.exception, ScanError)
        self.assertFalse(ctx.exception.rate_limited)

    def test_scan_image_rate_limited_stderr_raises_scan_error_with_rate_limited_flag(self) -> None:
        stderr = "TOOMANYREQUESTS: You have reached your unauthenticated pull rate limit."
        with patch(
            "check_container_cves.subprocess.run",
            return_value=_fake_run_result({}, returncode=1, stderr=stderr),
        ):
            with self.assertRaises(ScanError) as ctx:
                scan_image("trivy", "http://127.0.0.1:1", "postgres:17", "CRITICAL,HIGH")

        self.assertTrue(ctx.exception.rate_limited)


class ScanAllImagesTest(unittest.TestCase):
    def test_scan_all_images_returns_result_per_entry(self) -> None:
        vuln_payload = {"Results": [{"Vulnerabilities": [{"Severity": "HIGH"}]}]}

        def fake_run(command, capture_output, text):
            image_ref = command[-1]
            payload = _EMPTY_RESULT if image_ref.startswith("mcr.microsoft.com") else vuln_payload
            return _fake_run_result(payload)

        with patch("check_container_cves.subprocess.run", side_effect=fake_run):
            scan_results = scan_all_images("trivy", "http://127.0.0.1:1", _ENTRIES, "CRITICAL,HIGH")

        self.assertEqual(set(scan_results), {"postgres", "mssql"})
        self.assertEqual(scan_results["mssql"], _EMPTY_RESULT)
        self.assertEqual(scan_results["postgres"], vuln_payload)

    def test_scan_all_images_one_scan_error_does_not_crash_the_batch(self) -> None:
        stderr = "TOOMANYREQUESTS: You have reached your unauthenticated pull rate limit."

        def fake_run(command, capture_output, text):
            image_ref = command[-1]
            if image_ref.startswith("mcr.microsoft.com"):
                return _fake_run_result({}, returncode=1, stderr=stderr)
            return _fake_run_result(_EMPTY_RESULT)

        with patch("check_container_cves.subprocess.run", side_effect=fake_run):
            scan_results = scan_all_images("trivy", "http://127.0.0.1:1", _ENTRIES, "CRITICAL,HIGH")

        self.assertEqual(scan_results["postgres"], _EMPTY_RESULT)
        self.assertIsInstance(scan_results["mssql"], ScanError)
        self.assertTrue(scan_results["mssql"].rate_limited)


class TrivyServerLifecycleTest(unittest.TestCase):
    def test_start_trivy_server_waits_for_healthz_then_returns_url(self) -> None:
        fake_process = MagicMock()
        fake_process.poll.return_value = None

        with patch(
            "check_container_cves.subprocess.Popen", return_value=fake_process
        ) as popen_mock, patch(
            "check_container_cves.urllib.request.urlopen",
            return_value=_fake_healthz_response(),
        ):
            process, server_url = start_trivy_server("trivy")

        self.assertIs(process, fake_process)
        self.assertTrue(server_url.startswith("http://127.0.0.1:"))
        popen_mock.assert_called_once()

    def test_start_trivy_server_exits_early_raises(self) -> None:
        fake_process = MagicMock()
        fake_process.poll.return_value = 1
        fake_process.returncode = 1
        fake_process.stderr = io.StringIO("boom")

        with patch("check_container_cves.subprocess.Popen", return_value=fake_process):
            with self.assertRaisesRegex(RuntimeError, "exited early"):
                start_trivy_server("trivy")

        fake_process.terminate.assert_called_once()

    def test_start_trivy_server_never_ready_raises_after_timeout(self) -> None:
        fake_process = MagicMock()
        fake_process.poll.return_value = None

        with patch(
            "check_container_cves.subprocess.Popen", return_value=fake_process
        ), patch(
            "check_container_cves.urllib.request.urlopen",
            side_effect=urllib.error.URLError("no network"),
        ), patch("check_container_cves.SERVER_READY_TIMEOUT_SECONDS", 0.2):
            with self.assertRaisesRegex(RuntimeError, "did not become ready"):
                start_trivy_server("trivy")

        fake_process.terminate.assert_called_once()

    def test_stop_trivy_server_terminates_and_waits(self) -> None:
        fake_process = MagicMock()

        stop_trivy_server(fake_process)

        fake_process.terminate.assert_called_once()
        fake_process.wait.assert_called_once_with(timeout=10)
        fake_process.kill.assert_not_called()

    def test_stop_trivy_server_wait_timeout_kills_process(self) -> None:
        fake_process = MagicMock()
        fake_process.wait.side_effect = [subprocess.TimeoutExpired(cmd="trivy", timeout=10), None]

        stop_trivy_server(fake_process)

        fake_process.kill.assert_called_once()


class MainTest(unittest.TestCase):
    def test_main_all_images_clean_returns_zero(self) -> None:
        fake_process = MagicMock()
        fake_process.poll.return_value = None

        with patch(
            "check_container_cves.load_container_defaults", return_value=_ENTRIES
        ), patch("check_container_cves.find_trivy_bin", return_value="trivy"), patch(
            "check_container_cves.subprocess.Popen", return_value=fake_process
        ), patch(
            "check_container_cves.subprocess.run",
            return_value=_fake_run_result(_EMPTY_RESULT),
        ), patch(
            "check_container_cves.urllib.request.urlopen",
            return_value=_fake_healthz_response(),
        ):
            exit_code = main()

        self.assertEqual(exit_code, 0)
        fake_process.terminate.assert_called_once()

    def test_main_failing_image_returns_zero_and_stops_server(self) -> None:
        fake_process = MagicMock()
        fake_process.poll.return_value = None
        vuln_payload = {
            "Results": [
                {"Vulnerabilities": [{"VulnerabilityID": "CVE-2026-9999", "Severity": "HIGH"}]}
            ]
        }

        def fake_run(command, capture_output, text):
            image_ref = command[-1]
            if image_ref == "postgres:17":
                return _fake_run_result(vuln_payload)
            return _fake_run_result(_EMPTY_RESULT)

        output = io.StringIO()
        with patch(
            "check_container_cves.load_container_defaults", return_value=_ENTRIES
        ), patch("check_container_cves.find_trivy_bin", return_value="trivy"), patch(
            "check_container_cves.subprocess.Popen", return_value=fake_process
        ), patch("check_container_cves.subprocess.run", side_effect=fake_run), patch(
            "check_container_cves.urllib.request.urlopen",
            return_value=_fake_healthz_response(),
        ), patch.dict(os.environ, {}, clear=False), contextlib.redirect_stdout(output):
            os.environ.pop("ARENA_CONTAINER_CVE_SHOW_IDS", None)
            exit_code = main()

        self.assertEqual(exit_code, 0)
        fake_process.terminate.assert_called_once()
        self.assertNotIn("CVE-2026-9999", output.getvalue())

    def test_main_failing_image_with_show_ids_flag_prints_cve_ids(self) -> None:
        fake_process = MagicMock()
        fake_process.poll.return_value = None
        vuln_payload = {
            "Results": [
                {"Vulnerabilities": [{"VulnerabilityID": "CVE-2026-9999", "Severity": "HIGH"}]}
            ]
        }

        def fake_run(command, capture_output, text):
            image_ref = command[-1]
            if image_ref == "postgres:17":
                return _fake_run_result(vuln_payload)
            return _fake_run_result(_EMPTY_RESULT)

        output = io.StringIO()
        with patch(
            "check_container_cves.load_container_defaults", return_value=_ENTRIES
        ), patch("check_container_cves.find_trivy_bin", return_value="trivy"), patch(
            "check_container_cves.subprocess.Popen", return_value=fake_process
        ), patch("check_container_cves.subprocess.run", side_effect=fake_run), patch(
            "check_container_cves.urllib.request.urlopen",
            return_value=_fake_healthz_response(),
        ), patch.dict(
            os.environ, {"ARENA_CONTAINER_CVE_SHOW_IDS": "true"}
        ), contextlib.redirect_stdout(output):
            exit_code = main()

        self.assertEqual(exit_code, 0)
        self.assertIn("CVE-2026-9999", output.getvalue())

    def test_main_rate_limited_scan_aborts_before_building_table(self) -> None:
        fake_process = MagicMock()
        fake_process.poll.return_value = None
        stderr = "TOOMANYREQUESTS: You have reached your unauthenticated pull rate limit."

        def fake_run(command, capture_output, text):
            image_ref = command[-1]
            if image_ref == "postgres:17":
                return _fake_run_result({}, returncode=1, stderr=stderr)
            return _fake_run_result(_EMPTY_RESULT)

        with patch(
            "check_container_cves.load_container_defaults", return_value=_ENTRIES
        ), patch("check_container_cves.find_trivy_bin", return_value="trivy"), patch(
            "check_container_cves.subprocess.Popen", return_value=fake_process
        ), patch("check_container_cves.subprocess.run", side_effect=fake_run), patch(
            "check_container_cves.urllib.request.urlopen",
            return_value=_fake_healthz_response(),
        ):
            exit_code = main()

        self.assertEqual(exit_code, 1)
        fake_process.terminate.assert_called_once()

    def test_main_mixed_rate_limited_and_real_error_reports_both(self) -> None:
        fake_process = MagicMock()
        fake_process.poll.return_value = None
        rate_limit_stderr = "TOOMANYREQUESTS: You have reached your unauthenticated pull rate limit."

        def fake_run(command, capture_output, text):
            image_ref = command[-1]
            if image_ref == "postgres:17":
                return _fake_run_result({}, returncode=1, stderr=rate_limit_stderr)
            return _fake_run_result({}, returncode=1, stderr="connection reset")

        output = io.StringIO()
        with patch(
            "check_container_cves.load_container_defaults", return_value=_ENTRIES
        ), patch("check_container_cves.find_trivy_bin", return_value="trivy"), patch(
            "check_container_cves.subprocess.Popen", return_value=fake_process
        ), patch("check_container_cves.subprocess.run", side_effect=fake_run), patch(
            "check_container_cves.urllib.request.urlopen",
            return_value=_fake_healthz_response(),
        ), contextlib.redirect_stdout(output):
            exit_code = main()

        self.assertEqual(exit_code, 1)
        printed = output.getvalue()
        self.assertIn("postgres", printed)
        self.assertIn("rate limited", printed)
        self.assertIn("mssql", printed)
        self.assertIn("connection reset", printed)


if __name__ == "__main__":
    unittest.main()
