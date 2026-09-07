using System;
using System.Linq;
using ArenaDotnet.Xunit.Support;
using Xunit;

namespace ArenaDotnet.Xunit.UnitTest;

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

    [Theory]
    [InlineData("oracle")]
    [InlineData("broker")]
    [InlineData("server")]
    [InlineData("kafka1")]
    public void Build_SixCharacterName_AppendsSuffix(string name)
    {
        var id = ArenaIdentifiers.Build("arena-postgres", name);

        Assert.StartsWith($"arena-postgres-{name}-", id);
        Assert.NotEqual(name, id);
    }

    [Fact]
    public void Build_AlreadyBuiltIdentifier_ReturnsItUnchanged()
    {
        var once = ArenaIdentifiers.Build("arena-postgres", "orders");

        Assert.Equal(once, ArenaIdentifiers.Build("arena-postgres", once));
    }

    [Fact]
    public void Build_IdentifierBuiltByAnotherModule_IsPreserved()
    {
        const string built = "arena-oracle-api-oracle-a1b2c3";

        Assert.Equal(built, ArenaIdentifiers.Build("arena-postgres", built));
    }
}
