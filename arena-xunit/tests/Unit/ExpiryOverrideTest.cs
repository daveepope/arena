using System;
using ArenaDotnet.Xunit.Component;
using ArenaDotnet.Xunit.Dep;
using Newtonsoft.Json.Linq;
using Xunit;

namespace ArenaDotnet.Xunit.UnitTest;

public class ExpiryOverrideTest
{
    private sealed record BuilderCase(
        Func<string> Defaults,
        Func<TimeSpan, string> WithExpiry,
        Func<string> WithoutExpiry);

    public static TheoryData<string> Builders() =>
        new()
        {
            "http",
            "kafka",
            "localstack",
            "mssql",
            "oracle",
            "postgres",
            "smtp",
            "temporal",
            "containerized-component",
        };

    private static BuilderCase Case(string builder) =>
        builder switch
        {
            "http" => new BuilderCase(
                () => new HttpDependencyBuilder("orders").Build().ForFfi(),
                e => new HttpDependencyBuilder("orders").WithExpiry(e).Build().ForFfi(),
                () => new HttpDependencyBuilder("orders").WithoutExpiry().Build().ForFfi()),
            "kafka" => new BuilderCase(
                () => new KafkaDependencyBuilder("orders").Build().ForFfi(),
                e => new KafkaDependencyBuilder("orders").WithExpiry(e).Build().ForFfi(),
                () => new KafkaDependencyBuilder("orders").WithoutExpiry().Build().ForFfi()),
            "localstack" => new BuilderCase(
                () => new LocalstackDependencyBuilder("orders").Build().ForFfi(),
                e => new LocalstackDependencyBuilder("orders").WithExpiry(e).Build().ForFfi(),
                () => new LocalstackDependencyBuilder("orders").WithoutExpiry().Build().ForFfi()),
            "mssql" => new BuilderCase(
                () => new MssqlDependencyBuilder("orders").Build().ForFfi(),
                e => new MssqlDependencyBuilder("orders").WithExpiry(e).Build().ForFfi(),
                () => new MssqlDependencyBuilder("orders").WithoutExpiry().Build().ForFfi()),
            "oracle" => new BuilderCase(
                () => new OracleDependencyBuilder("orders").Build().ForFfi(),
                e => new OracleDependencyBuilder("orders").WithExpiry(e).Build().ForFfi(),
                () => new OracleDependencyBuilder("orders").WithoutExpiry().Build().ForFfi()),
            "postgres" => new BuilderCase(
                () => new PostgresDependencyBuilder("orders").Build().ForFfi(),
                e => new PostgresDependencyBuilder("orders").WithExpiry(e).Build().ForFfi(),
                () => new PostgresDependencyBuilder("orders").WithoutExpiry().Build().ForFfi()),
            "smtp" => new BuilderCase(
                () => new SmtpDependencyBuilder("orders").Build().ForFfi(),
                e => new SmtpDependencyBuilder("orders").WithExpiry(e).Build().ForFfi(),
                () => new SmtpDependencyBuilder("orders").WithoutExpiry().Build().ForFfi()),
            "temporal" => new BuilderCase(
                () => new TemporalDependencyBuilder("orders").Build().ForFfi(),
                e => new TemporalDependencyBuilder("orders").WithExpiry(e).Build().ForFfi(),
                () => new TemporalDependencyBuilder("orders").WithoutExpiry().Build().ForFfi()),
            "containerized-component" => new BuilderCase(
                () => ContainerizedComponent().Build().ForFfi(),
                e => ContainerizedComponent().WithExpiry(e).Build().ForFfi(),
                () => ContainerizedComponent().WithoutExpiry().Build().ForFfi()),
            _ => throw new InvalidOperationException($"unknown builder: {builder}"),
        };

    private static ContainerizedComponentBuilder ContainerizedComponent() =>
        new ContainerizedComponentBuilder("web").WithContainerfile("Containerfile");

    [Theory]
    [MemberData(nameof(Builders))]
    public void Build_WithoutExpiryOverride_OmitsExpirySeconds(string builder)
    {
        var obj = JObject.Parse(Case(builder).Defaults());

        Assert.Null(obj["expiry_seconds"]);
    }

    [Theory]
    [MemberData(nameof(Builders))]
    public void WithExpiry_ThirtySeconds_SetsExpirySeconds(string builder)
    {
        var obj = JObject.Parse(Case(builder).WithExpiry(TimeSpan.FromSeconds(30)));

        Assert.Equal(30, (long)obj["expiry_seconds"]!);
    }

    [Theory]
    [MemberData(nameof(Builders))]
    public void WithoutExpiry_Called_SetsExpirySecondsToZero(string builder)
    {
        var obj = JObject.Parse(Case(builder).WithoutExpiry());

        Assert.Equal(0, (long)obj["expiry_seconds"]!);
    }

    [Theory]
    [MemberData(nameof(Builders))]
    public void WithExpiry_SubSecond_ClampsToOneSecond(string builder)
    {
        var obj = JObject.Parse(Case(builder).WithExpiry(TimeSpan.FromMilliseconds(500)));

        Assert.Equal(1, (long)obj["expiry_seconds"]!);
    }

    [Theory]
    [MemberData(nameof(Builders))]
    public void WithExpiry_Zero_SetsExpirySecondsToZero(string builder)
    {
        var obj = JObject.Parse(Case(builder).WithExpiry(TimeSpan.Zero));

        Assert.Equal(0, (long)obj["expiry_seconds"]!);
    }

    [Theory]
    [MemberData(nameof(Builders))]
    public void WithExpiry_Negative_ThrowsArgumentOutOfRangeException(string builder)
    {
        var expiry = Case(builder).WithExpiry;

        Assert.Throws<ArgumentOutOfRangeException>(() => expiry(TimeSpan.FromMilliseconds(-500)));
    }
}
