using System;
using ArenaDotnet.Xunit.Component;
using ArenaDotnet.Xunit.Dep;
using Newtonsoft.Json.Linq;
using Xunit;

namespace ArenaDotnet.Xunit.UnitTest;

public class ExpiryOverrideTest
{
    [Fact]
    public void Build_WithoutExpiryOverride_OmitsExpirySeconds()
    {
        var obj = JObject.Parse(new PostgresDependencyBuilder("orders").Build().ForFfi());

        Assert.Null(obj["expiry_seconds"]);
    }

    [Fact]
    public void WithExpiry_ThirtySeconds_SetsExpirySeconds()
    {
        var obj = JObject.Parse(
            new PostgresDependencyBuilder("orders")
                .WithExpiry(TimeSpan.FromSeconds(30))
                .Build()
                .ForFfi());

        Assert.Equal(30, (long)obj["expiry_seconds"]!);
    }

    [Fact]
    public void WithoutExpiry_Called_SetsExpirySecondsToZero()
    {
        var obj = JObject.Parse(
            new PostgresDependencyBuilder("orders").WithoutExpiry().Build().ForFfi());

        Assert.Equal(0, (long)obj["expiry_seconds"]!);
    }

    [Fact]
    public void WithExpiry_ContainerizedComponent_SetsExpirySeconds()
    {
        var obj = JObject.Parse(
            new ContainerizedComponentBuilder("web")
                .WithContainerfile("Containerfile")
                .WithExpiry(TimeSpan.FromSeconds(45))
                .Build()
                .ForFfi());

        Assert.Equal(45, (long)obj["expiry_seconds"]!);
    }

    [Fact]
    public void WithExpiry_SubSecond_ClampsToOneSecond()
    {
        var obj = JObject.Parse(
            new PostgresDependencyBuilder("orders")
                .WithExpiry(TimeSpan.FromMilliseconds(500))
                .Build()
                .ForFfi());

        Assert.Equal(1, (long)obj["expiry_seconds"]!);
    }

    [Fact]
    public void WithExpiry_Negative_ThrowsArgumentOutOfRangeException()
    {
        Assert.Throws<ArgumentOutOfRangeException>(
            () => new PostgresDependencyBuilder("orders").WithExpiry(TimeSpan.FromMilliseconds(-500)));
    }

    [Fact]
    public void WithExpiry_Zero_SetsExpirySecondsToZero()
    {
        var obj = JObject.Parse(
            new PostgresDependencyBuilder("orders").WithExpiry(TimeSpan.Zero).Build().ForFfi());

        Assert.Equal(0, (long)obj["expiry_seconds"]!);
    }
}
