import unittest

from generate_nuget_sbom import build_sbom, parse_nuget_archives


class ParseNugetArchivesTest(unittest.TestCase):
    def test_parse_nuget_archives_singleBlock_returnsIdVersion(self) -> None:
        text = """
nuget_archive(
    name = "newtonsoft_json",
    id = "Newtonsoft.Json",
    sources = ["https://api.nuget.org/v3/index.json"],
    version = "13.0.4",
)
"""
        self.assertEqual(parse_nuget_archives(text), {("Newtonsoft.Json", "13.0.4")})


class BuildSbomTest(unittest.TestCase):
    def test_build_sbom_nugetArchive_includesComponent(self) -> None:
        text = """
nuget_archive(
    name = "newtonsoft_json",
    id = "Newtonsoft.Json",
    sources = ["https://api.nuget.org/v3/index.json"],
    version = "13.0.4",
)
"""
        sbom = build_sbom(text)
        self.assertEqual(sbom["bomFormat"], "CycloneDX")
        self.assertEqual(
            sbom["components"],
            [
                {
                    "type": "library",
                    "name": "Newtonsoft.Json",
                    "version": "13.0.4",
                    "purl": "pkg:nuget/Newtonsoft.Json@13.0.4",
                }
            ],
        )


if __name__ == "__main__":
    unittest.main()
