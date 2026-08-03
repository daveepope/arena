using System;
using System.Linq;
using ArenaXunit.Support;
using Xunit;

namespace ArenaXunit.UnitTest;

public class IdentifierTest
{
    [Fact]
    public void Build_SingleName_ReturnsValidIdentifier()
    {
        var id = ArenaIdentifiers.Build("arena-http", "stub");
        Assert.StartsWith("arena-http-", id);
        Assert.Contains("-", id);
        Assert.True(id.Split('-').Length >= 3);
    }

    [Fact]
    public void Build_SameInput_ReturnsDifferentSuffixes()
    {
        var id1 = ArenaIdentifiers.Build("arena-http", "stub");
        var id2 = ArenaIdentifiers.Build("arena-http", "stub");
        Assert.NotEqual(id1, id2);
        Assert.Equal(id1.Substring(0, id1.LastIndexOf('-')), id2.Substring(0, id2.LastIndexOf('-')));
    }

    [Fact]
    public void Build_SpecialChars_ConvertedToSlug()
    {
        var id = ArenaIdentifiers.Build("arena-http", "my-stub-test");
        Assert.Contains("my-stub-test", id);
    }

    [Fact]
    public void Build_EmptyName_ReturnsIdentifier()
    {
        var id = ArenaIdentifiers.Build("arena-http", "");
        Assert.StartsWith("arena-http-", id);
    }

    [Fact]
    public void Build_ModulePrefix_Preserved()
    {
        var id1 = ArenaIdentifiers.Build("arena-kafka", "test");
        var id2 = ArenaIdentifiers.Build("arena-postgres", "test");
        Assert.StartsWith("arena-kafka-", id1);
        Assert.StartsWith("arena-postgres-", id2);
    }
}
