using System;
using System.Linq;
using ArenaDotnet.Xunit.Dep;
using ArenaDotnet.Xunit.Support;
using Newtonsoft.Json.Linq;
using Xunit;

namespace ArenaDotnet.Xunit.UnitTest;

public class OracleDependencyBuilderSerializationTest
{
    [Fact]
    public void Build_DefaultPort_SerializesCorrectJson()
    {
        var dep = new OracleDependencyBuilder("test").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("oracle", obj["type"]);
        Assert.Equal(1521, obj["port"]);
        Assert.NotNull(obj["identifier"]);
    }

    [Fact]
    public void Build_CustomPort_SerializesCorrectJson()
    {
        var dep = new OracleDependencyBuilder("test").WithPort(1522).Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal(1522, obj["port"]);
    }

    [Fact]
    public void Build_Identifier_MatchesPattern()
    {
        var dep = new OracleDependencyBuilder("test").Build();
        Assert.StartsWith("arena-oracle-", dep.Identifier);
    }

    [Fact]
    public void Build_WithImageName_SerializesCorrectJson()
    {
        var dep = new OracleDependencyBuilder("test").WithImageName("oracle-free").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("oracle-free", obj["image_name"]);
    }

    [Fact]
    public void Build_WithImage_SerializesCorrectJson()
    {
        var dep = new OracleDependencyBuilder("test").WithImage("21-slim").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("21-slim", obj["image"]);
    }

    [Fact]
    public void Build_WithContainerName_SerializesCorrectJson()
    {
        var dep = new OracleDependencyBuilder("test").WithContainerName("my-oracle").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("my-oracle", obj["container_name"]);
    }

    [Fact]
    public void Build_WithAdminPassword_SerializesCorrectJson()
    {
        var dep = new OracleDependencyBuilder("test").WithAdminPassword("secret-admin").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("secret-admin", obj["admin_password"]);
    }

    [Fact]
    public void Build_WithDatabaseFieldsAndStartupScripts_SerializesCorrectJson()
    {
        var dep = new OracleDependencyBuilder("test")
            .WithDatabaseName("arena")
            .WithDatabaseUsername("arena_user")
            .WithDatabasePassword("secret")
            .WithStartupSqlScripts(new[] { "seed.sql", "grants.sql" })
            .Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("arena", obj["database_name"]);
        Assert.Equal("arena_user", obj["database_username"]);
        Assert.Equal("secret", obj["database_password"]);
        var scripts = Assert.IsType<JArray>(obj["startup_sql_scripts"]);
        Assert.Equal(2, scripts.Count);
        Assert.Equal("seed.sql", scripts[0]);
        Assert.Equal("grants.sql", scripts[1]);
    }
}
