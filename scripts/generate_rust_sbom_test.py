import unittest

from generate_rust_sbom import build_sbom


class BuildSbomTest(unittest.TestCase):
    def test_build_sbom_externalCrate_includesComponent(self) -> None:
        lock = {
            "crates": {
                "serde 1.0.210": {
                    "name": "serde",
                    "version": "1.0.210",
                    "package_url": "https://github.com/serde-rs/serde",
                },
            }
        }
        sbom = build_sbom(lock)
        self.assertEqual(sbom["bomFormat"], "CycloneDX")
        self.assertEqual(
            sbom["components"],
            [{"type": "library", "name": "serde", "version": "1.0.210", "purl": "pkg:cargo/serde@1.0.210"}],
        )

    def test_build_sbom_localWorkspaceCrate_excludesComponent(self) -> None:
        lock = {
            "crates": {
                "arena 6.0.2": {
                    "name": "arena",
                    "version": "6.0.2",
                    "package_url": None,
                },
            }
        }
        sbom = build_sbom(lock)
        self.assertEqual(sbom["components"], [])


if __name__ == "__main__":
    unittest.main()
