import pytest

from arena_pytest import BuildTool
from arena_pytest.exec.executable_component import ExecutableComponentBuilder


@pytest.mark.parametrize(
    "build_tool,expected",
    [
        (BuildTool.CARGO, "cargo"),
        (BuildTool.MAVEN, "maven"),
        (BuildTool.GRADLE, "gradle"),
        (BuildTool.DOTNET, "dotnet"),
        (BuildTool.MAKE, "make"),
        (BuildTool.CMAKE, "cmake"),
        (BuildTool.PYTHON, "python"),
    ],
)
def test_with_build_tool_known_variant_serializes_tool_value(build_tool, expected):
    component = ExecutableComponentBuilder("worker").with_build_tool(build_tool).build()

    assert component._for_ffi()["build_tool"] == expected


def test_with_build_tool_custom_serializes_command_and_args():
    component = (
        ExecutableComponentBuilder("worker")
        .with_build_tool_custom("./build.sh", ["--release"])
        .build()
    )

    assert component._for_ffi()["build_tool"] == {
        "command": "./build.sh",
        "args": ["--release"],
    }
