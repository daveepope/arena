"""Converts a Cobertura coverage XML file into an lcov tracefile.

rules_dotnet has no coverage instrumentation support (see
https://github.com/bazel-contrib/rules_dotnet/issues/359), so `bazel coverage`
gets nothing for csharp_test targets on its own. `dotnet_coverage_test`
(see defs.bzl) collects real coverage with `dotnet-coverage collect -f
cobertura` instead and uses this script to turn that Cobertura XML into the
lcov format that `bazel coverage --combined_report=lcov` and Codecov expect.
"""

import sys
import xml.etree.ElementTree as ET


def convert(cobertura_path: str, lcov_path: str) -> None:
    tree = ET.parse(cobertura_path)
    lines_by_file: dict[str, dict[int, int]] = {}

    for cls in tree.getroot().iter("class"):
        filename = cls.get("filename")
        if not filename:
            continue
        if filename.startswith("./"):
            filename = filename[2:]

        lines_elem = cls.find("lines")
        if lines_elem is None:
            continue

        file_lines = lines_by_file.setdefault(filename, {})
        for line in lines_elem.findall("line"):
            number = int(line.get("number"))
            hits = int(line.get("hits", "0"))
            file_lines[number] = max(file_lines.get(number, 0), hits)

    with open(lcov_path, "w") as out:
        for filename in sorted(lines_by_file):
            file_lines = lines_by_file[filename]
            out.write("SF:%s\n" % filename)
            lines_hit = 0
            for number in sorted(file_lines):
                hits = file_lines[number]
                if hits > 0:
                    lines_hit += 1
                out.write("DA:%d,%d\n" % (number, hits))
            out.write("LH:%d\n" % lines_hit)
            out.write("LF:%d\n" % len(file_lines))
            out.write("end_of_record\n")


def main() -> None:
    if len(sys.argv) != 3:
        sys.exit("usage: cobertura_to_lcov.py <cobertura.xml> <output.lcov>")
    convert(sys.argv[1], sys.argv[2])


if __name__ == "__main__":
    main()
