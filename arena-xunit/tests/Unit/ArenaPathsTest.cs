using System;
using System.IO;
using ArenaDotnet.Xunit.Ffi;
using Xunit;

namespace ArenaDotnet.Xunit.UnitTest;

public class ArenaPathsTest : IDisposable
{
    private readonly string _tempDir;

    public ArenaPathsTest()
    {
        _tempDir = Directory.CreateTempSubdirectory("arena_paths_test_").FullName;
    }

    public void Dispose()
    {
        Directory.Delete(_tempDir, recursive: true);
    }

    [Fact]
    public void ResolveViaDepsFile_DirectoryDoesNotExist_ReturnsNull()
    {
        var missingDir = Path.Combine(_tempDir, "does-not-exist");
        Assert.Null(ArenaPaths.ResolveViaDepsFile(missingDir));
    }

    [Fact]
    public void ResolveViaDepsFile_NullDirectory_ReturnsNull()
    {
        Assert.Null(ArenaPaths.ResolveViaDepsFile(null));
    }

    [Fact]
    public void ResolveViaDepsFile_NoDepsJsonInDirectory_ReturnsNull()
    {
        Assert.Null(ArenaPaths.ResolveViaDepsFile(_tempDir));
    }

    [Fact]
    public void ResolveViaDepsFile_MalformedDepsJson_DoesNotThrowAndReturnsNull()
    {
        File.WriteAllText(Path.Combine(_tempDir, "consumer.deps.json"), "not valid json");

        var exception = Record.Exception(() => ArenaPaths.ResolveViaDepsFile(_tempDir));

        Assert.Null(exception);
        Assert.Null(ArenaPaths.ResolveViaDepsFile(_tempDir));
    }

    [Fact]
    public void ResolveViaDepsFile_ValidDepsJsonWithoutNativeAsset_ReturnsNull()
    {
        File.WriteAllText(Path.Combine(_tempDir, "consumer.deps.json"), """
        {
          "runtimeTarget": { "name": ".NETCoreApp,Version=v8.0" },
          "compilationOptions": {},
          "targets": {
            ".NETCoreApp,Version=v8.0": {
              "consumer/1.0.0": { "runtime": { "consumer.dll": {} } }
            }
          },
          "libraries": {
            "consumer/1.0.0": { "type": "project", "serviceable": false, "sha512": "" }
          }
        }
        """);

        Assert.Null(ArenaPaths.ResolveViaDepsFile(_tempDir));
    }

    [Fact]
    public void ResolveViaDepsFile_MultipleDepsJsonOneMalformed_SkipsItAndKeepsLooking()
    {
        File.WriteAllText(Path.Combine(_tempDir, "aaa_broken.deps.json"), "{ not json");
        File.WriteAllText(Path.Combine(_tempDir, "zzz_valid.deps.json"), """
        {
          "runtimeTarget": { "name": ".NETCoreApp,Version=v8.0" },
          "compilationOptions": {},
          "targets": { ".NETCoreApp,Version=v8.0": {} },
          "libraries": {}
        }
        """);

        var exception = Record.Exception(() => ArenaPaths.ResolveViaDepsFile(_tempDir));

        Assert.Null(exception);
        Assert.Null(ArenaPaths.ResolveViaDepsFile(_tempDir));
    }
}
