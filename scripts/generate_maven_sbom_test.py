import unittest

from generate_maven_sbom import build_sbom


class BuildSbomTest(unittest.TestCase):
    def test_build_sbom_artifact_includesComponent(self) -> None:
        lock = {
            "artifacts": {
                "com.fasterxml.jackson.core:jackson-databind": {
                    "version": "2.22.2",
                    "shasums": {"jar": "abc123"},
                },
            }
        }
        sbom = build_sbom(lock)
        self.assertEqual(sbom["bomFormat"], "CycloneDX")
        self.assertEqual(
            sbom["components"],
            [
                {
                    "type": "library",
                    "name": "com.fasterxml.jackson.core:jackson-databind",
                    "version": "2.22.2",
                    "purl": "pkg:maven/com.fasterxml.jackson.core/jackson-databind@2.22.2",
                }
            ],
        )


if __name__ == "__main__":
    unittest.main()
