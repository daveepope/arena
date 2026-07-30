using System;
using System.Linq;
using ArenaXunit.Support;
using Xunit;

namespace ArenaXunit.UnitTest;

public class IdentifierTest
{
    [Fact]
    public void build_single_name_returns_valid_identifier()
    {
        var id = ArenaIdentifiers.Build("arena-http", "stub");
        Assert.StartsWith("arena-http-", id);
        Assert.Contains("-", id);
        Assert.Equal(3, id.Split('-').Length);
    }

    [Fact]
    public void build_same_input_returns_different_suffixes()
    {
        var id1 = ArenaIdentifiers.Build("arena-http", "stub");
        var id2 = ArenaIdentifiers.Build("arena-http", "stub");
        Assert.NotEqual(id1, id2);
        Assert.Equal(id1.Substring(0, id1.LastIndexOf('-')), id2.Substring(0, id2.LastIndexOf('-')));
    }

    [Fact]
    public void build_special_chars_converted_to_slug()
    {
        var id = ArenaIdentifiers.Build("arena-http", "my-stub-test");
        Assert.Contains("my-stub-test", id);
    }

    [Fact]
    public void build_empty_name_returns_identifier()
    {
        var id = ArenaIdentifiers.Build("arena-http", "");
        Assert.StartsWith("arena-http-", id);
    }

    [Fact]
    public void build_module_prefix_preserved()
    {
        var id1 = ArenaIdentifiers.Build("arena-kafka", "test");
        var id2 = ArenaIdentifiers.Build("arena-postgres", "test");
        Assert.StartsWith("arena-kafka-", id1);
        Assert.StartsWith("arena-postgres-", id2);
    }
}
