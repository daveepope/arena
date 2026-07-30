using System;
using System.Linq;
using ArenaXunit.Dep;
using ArenaXunit.Support;
using Newtonsoft.Json.Linq;
using Xunit;

namespace ArenaXunit.UnitTest;

public class MssqlDependencyBuilderSerializationTest
{
    [Fact]
    public void build_default_port_serializes_correct_json()
    {
        var dep = new MssqlDependencyBuilder("test").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("mssql", obj["type"]);
        Assert.Equal(1433, obj["port"]);
        Assert.NotNull(obj["identifier"]);
    }

    [Fact]
    public void build_custom_port_serializes_correct_json()
    {
        var dep = new MssqlDependencyBuilder("test").WithPort(1533).Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal(1533, obj["port"]);
    }

    [Fact]
    public void build_encryption_off_serializes_correct_json()
    {
        var dep = new MssqlDependencyBuilder("test").WithEncryption(MssqlEncryption.Off).Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("off", obj["encryption"]);
    }

    [Fact]
    public void build_encryption_on_serializes_correct_json()
    {
        var dep = new MssqlDependencyBuilder("test").WithEncryption(MssqlEncryption.On).Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("on", obj["encryption"]);
    }

    [Fact]
    public void build_encryption_strict_serializes_correct_json()
    {
        var dep = new MssqlDependencyBuilder("test").WithEncryption(MssqlEncryption.Strict).Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("strict", obj["encryption"]);
    }

    [Fact]
    public void build_identifier_matches_pattern()
    {
        var dep = new MssqlDependencyBuilder("test").Build();
        Assert.StartsWith("arena-mssql-", dep.Identifier);
    }
}
