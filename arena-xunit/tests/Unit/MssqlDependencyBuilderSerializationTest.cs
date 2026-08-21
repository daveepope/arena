using System;
using System.Linq;
using ArenaDotnet.Xunit.Dep;
using ArenaDotnet.Xunit.Support;
using Newtonsoft.Json.Linq;
using Xunit;

namespace ArenaDotnet.Xunit.UnitTest;

public class MssqlDependencyBuilderSerializationTest
{
    [Fact]
    public void Build_DefaultPort_SerializesCorrectJson()
    {
        var dep = new MssqlDependencyBuilder("test").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("mssql", obj["type"]);
        Assert.Equal(1433, obj["port"]);
        Assert.NotNull(obj["identifier"]);
    }

    [Fact]
    public void Build_CustomPort_SerializesCorrectJson()
    {
        var dep = new MssqlDependencyBuilder("test").WithPort(1533).Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal(1533, obj["port"]);
    }

    [Fact]
    public void Build_EncryptionOff_SerializesCorrectJson()
    {
        var dep = new MssqlDependencyBuilder("test").WithEncryption(MssqlEncryption.Off).Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("off", obj["encryption"]);
    }

    [Fact]
    public void Build_EncryptionOn_SerializesCorrectJson()
    {
        var dep = new MssqlDependencyBuilder("test").WithEncryption(MssqlEncryption.On).Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("on", obj["encryption"]);
    }

    [Fact]
    public void Build_Identifier_MatchesPattern()
    {
        var dep = new MssqlDependencyBuilder("test").Build();
        Assert.StartsWith("arena-mssql-", dep.Identifier);
    }

    [Fact]
    public void Build_WithImageName_SerializesCorrectJson()
    {
        var dep = new MssqlDependencyBuilder("test").WithImageName("mcr.microsoft.com/mssql/server").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("mcr.microsoft.com/mssql/server", obj["image_name"]);
    }

    [Fact]
    public void Build_WithImage_SerializesCorrectJson()
    {
        var dep = new MssqlDependencyBuilder("test").WithImage("2025-CU7-ubuntu-24.04").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("2025-CU7-ubuntu-24.04", obj["image"]);
    }

    [Fact]
    public void Build_WithContainerName_SerializesCorrectJson()
    {
        var dep = new MssqlDependencyBuilder("test").WithContainerName("my-mssql-container").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("my-mssql-container", obj["container_name"]);
    }

    [Fact]
    public void Build_NoImageOverrides_OmitsImageFieldsFromJson()
    {
        var dep = new MssqlDependencyBuilder("test").Build();
        var json = dep.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Null(obj["image_name"]);
        Assert.Null(obj["image"]);
        Assert.Null(obj["container_name"]);
    }
}
