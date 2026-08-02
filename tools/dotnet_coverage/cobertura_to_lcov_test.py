import tempfile
import unittest
from pathlib import Path

from cobertura_to_lcov import convert

_SINGLE_CLASS_COBERTURA = """<?xml version="1.0" encoding="utf-8"?>
<coverage line-rate="1" branch-rate="1">
  <packages>
    <package name="pkg">
      <classes>
        <class name="Arena.Foo" filename="./arena-xunit/src/Foo.cs">
          <methods />
          <lines>
            <line number="10" hits="2" branch="False" />
            <line number="11" hits="0" branch="False" />
          </lines>
        </class>
      </classes>
    </package>
  </packages>
</coverage>
"""

_TWO_CLASSES_SAME_FILE_COBERTURA = """<?xml version="1.0" encoding="utf-8"?>
<coverage line-rate="1" branch-rate="1">
  <packages>
    <package name="pkg">
      <classes>
        <class name="Arena.Foo" filename="./arena-xunit/src/Foo.cs">
          <methods />
          <lines>
            <line number="10" hits="0" branch="False" />
          </lines>
        </class>
        <class name="Arena.Foo+Nested" filename="./arena-xunit/src/Foo.cs">
          <methods />
          <lines>
            <line number="10" hits="3" branch="False" />
          </lines>
        </class>
      </classes>
    </package>
  </packages>
</coverage>
"""


class ConvertTest(unittest.TestCase):
    def test_convert_singleClassWithHitsAndMisses_writesSfDaLhLf(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            cobertura_path = Path(tmp) / "coverage.cobertura.xml"
            lcov_path = Path(tmp) / "lcov.info"
            cobertura_path.write_text(_SINGLE_CLASS_COBERTURA)

            convert(str(cobertura_path), str(lcov_path))

            self.assertEqual(
                lcov_path.read_text(),
                "SF:arena-xunit/src/Foo.cs\n"
                "DA:10,2\n"
                "DA:11,0\n"
                "LH:1\n"
                "LF:2\n"
                "end_of_record\n",
            )

    def test_convert_twoClassesSameFileAndLine_takesMaxHits(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            cobertura_path = Path(tmp) / "coverage.cobertura.xml"
            lcov_path = Path(tmp) / "lcov.info"
            cobertura_path.write_text(_TWO_CLASSES_SAME_FILE_COBERTURA)

            convert(str(cobertura_path), str(lcov_path))

            self.assertEqual(
                lcov_path.read_text(),
                "SF:arena-xunit/src/Foo.cs\nDA:10,3\nLH:1\nLF:1\nend_of_record\n",
            )

    def test_convert_noClasses_writesEmptyFile(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            cobertura_path = Path(tmp) / "coverage.cobertura.xml"
            lcov_path = Path(tmp) / "lcov.info"
            cobertura_path.write_text('<?xml version="1.0"?><coverage><packages /></coverage>')

            convert(str(cobertura_path), str(lcov_path))

            self.assertEqual(lcov_path.read_text(), "")


if __name__ == "__main__":
    unittest.main()
