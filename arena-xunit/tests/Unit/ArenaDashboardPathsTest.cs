using System;
using System.IO;
using ArenaDotnet.Xunit.Ffi;
using Xunit;

namespace ArenaDotnet.Xunit.Tests.Unit;

public class ArenaDashboardPathsTest
{
    [Fact]
    public void DirectoryHoldingDashboardExecutable_DirectoryWithExecutable_ReturnsDirectory()
    {
        var dir = Path.Combine(Path.GetTempPath(), "arena-dashboard-paths-" + Guid.NewGuid());
        Directory.CreateDirectory(dir);
        try
        {
            File.WriteAllText(Path.Combine(dir, ArenaPaths.PlatformDashboardExecutableNames()[0]), "");

            Assert.Equal(dir, ArenaPaths.DirectoryHoldingDashboardExecutable(dir));
        }
        finally
        {
            Directory.Delete(dir, true);
        }
    }

    [Fact]
    public void DirectoryHoldingDashboardExecutable_DirectoryWithoutExecutable_ReturnsNull()
    {
        var dir = Path.Combine(Path.GetTempPath(), "arena-dashboard-paths-" + Guid.NewGuid());
        Directory.CreateDirectory(dir);
        try
        {
            Assert.Null(ArenaPaths.DirectoryHoldingDashboardExecutable(dir));
        }
        finally
        {
            Directory.Delete(dir, true);
        }
    }

    [Theory]
    [InlineData(null)]
    [InlineData("")]
    public void DirectoryHoldingDashboardExecutable_MissingDirectory_ReturnsNull(string? dir)
    {
        Assert.Null(ArenaPaths.DirectoryHoldingDashboardExecutable(dir));
    }

    [Fact]
    public void PlatformDashboardExecutableNames_AnyPlatform_MatchesLauncherNaming()
    {
        var expected = new[]
        {
            "arena-dashboard-server-linux",
            "arena-dashboard-server-macos",
            "arena-dashboard-server.exe",
        };

        Assert.Contains(ArenaPaths.PlatformDashboardExecutableNames()[0], expected);
    }
}
