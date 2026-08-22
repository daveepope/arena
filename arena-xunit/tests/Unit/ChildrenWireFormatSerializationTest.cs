using System;
using System.Collections.Generic;
using ArenaDotnet.Xunit.Component;
using ArenaDotnet.Xunit.Dep;
using Newtonsoft.Json.Linq;
using Xunit;

namespace ArenaDotnet.Xunit.UnitTest;

public class ChildrenWireFormatSerializationTest
{
    [Fact]
    public void HttpDependency_NoChildren_OmitsChildrenKey()
    {
        var dep = new HttpDependencyBuilder("parent").Build();
        var obj = JObject.Parse(dep.ForFfi());
        Assert.Null(obj["children"]);
    }

    [Fact]
    public void HttpDependency_WithChildDependency_NestsChildConfig()
    {
        var child = new HttpDependencyBuilder("child").WithPort(9090).Build();
        var dep = new HttpDependencyBuilder("parent")
            .AddChildDependency(child)
            .Build();
        var obj = JObject.Parse(dep.ForFfi());
        var children = Assert.IsType<JArray>(obj["children"]);
        Assert.Single(children);
        Assert.Equal("http", children[0]["type"]);
        Assert.Equal(9090, children[0]["port"]);
        Assert.Equal(child.Identifier, children[0]["identifier"]);
    }

    [Fact]
    public void ExecutableComponent_NoChildren_OmitsChildrenKey()
    {
        var comp = new ExecutableComponentBuilder("parent").WithExecutablePath("/bin/true").Build();
        var obj = JObject.Parse(comp.ForFfi());
        Assert.Null(obj["children"]);
    }

    [Fact]
    public void ExecutableComponent_WithChildComponent_NestsChildConfig()
    {
        var child = new ExecutableComponentBuilder("child").WithExecutablePath("/bin/true").Build();
        var comp = new ExecutableComponentBuilder("parent")
            .WithExecutablePath("/bin/true")
            .AddChildComponent(child)
            .Build();
        var obj = JObject.Parse(comp.ForFfi());
        var children = Assert.IsType<JArray>(obj["children"]);
        Assert.Single(children);
        Assert.Equal("exec", children[0]["type"]);
    }

    [Fact]
    public void ContainerizedComponent_NoChildren_OmitsChildrenKey()
    {
        var comp = new ContainerizedComponentBuilder("parent").WithContainerfile("Dockerfile").Build();
        var obj = JObject.Parse(comp.ForFfi());
        Assert.Null(obj["children"]);
    }

    [Fact]
    public void ContainerizedComponent_WithChildComponent_NestsChildConfig()
    {
        var child = new ContainerizedComponentBuilder("child").WithContainerfile("Dockerfile").Build();
        var comp = new ContainerizedComponentBuilder("parent")
            .WithContainerfile("Dockerfile")
            .AddChildComponent(child)
            .Build();
        var obj = JObject.Parse(comp.ForFfi());
        var children = Assert.IsType<JArray>(obj["children"]);
        Assert.Single(children);
        Assert.Equal("container", children[0]["type"]);
    }

    public static IEnumerable<object[]> RemainingDependencyTypeFactories()
    {
        yield return new object[] { "kafka", (Func<IArenaDependency?, IArenaDependency>)
            (child => { var b = new KafkaDependencyBuilder("dep"); if (child != null) b.AddChildDependency(child); return b.Build(); }) };
        yield return new object[] { "localstack", (Func<IArenaDependency?, IArenaDependency>)
            (child => { var b = new LocalstackDependencyBuilder("dep"); if (child != null) b.AddChildDependency(child); return b.Build(); }) };
        yield return new object[] { "mssql", (Func<IArenaDependency?, IArenaDependency>)
            (child => { var b = new MssqlDependencyBuilder("dep"); if (child != null) b.AddChildDependency(child); return b.Build(); }) };
        yield return new object[] { "oracle", (Func<IArenaDependency?, IArenaDependency>)
            (child => { var b = new OracleDependencyBuilder("dep"); if (child != null) b.AddChildDependency(child); return b.Build(); }) };
        yield return new object[] { "oauth", (Func<IArenaDependency?, IArenaDependency>)
            (child => { var b = new OauthDependencyBuilder("dep"); if (child != null) b.AddChildDependency(child); return b.Build(); }) };
        yield return new object[] { "postgres", (Func<IArenaDependency?, IArenaDependency>)
            (child => { var b = new PostgresDependencyBuilder("dep"); if (child != null) b.AddChildDependency(child); return b.Build(); }) };
        yield return new object[] { "smtp", (Func<IArenaDependency?, IArenaDependency>)
            (child => { var b = new SmtpDependencyBuilder("dep"); if (child != null) b.AddChildDependency(child); return b.Build(); }) };
        yield return new object[] { "temporal", (Func<IArenaDependency?, IArenaDependency>)
            (child => { var b = new TemporalDependencyBuilder("dep"); if (child != null) b.AddChildDependency(child); return b.Build(); }) };
    }

    [Theory]
    [MemberData(nameof(RemainingDependencyTypeFactories))]
    public void Dependency_NoChildren_OmitsChildrenKey(
        string expectedType, Func<IArenaDependency?, IArenaDependency> factory)
    {
        var dep = factory(null);
        var obj = JObject.Parse(dep.ForFfi());
        Assert.Null(obj["children"]);
    }

    [Theory]
    [MemberData(nameof(RemainingDependencyTypeFactories))]
    public void Dependency_WithChild_NestsChildConfig(
        string expectedType, Func<IArenaDependency?, IArenaDependency> factory)
    {
        var child = factory(null);
        var dep = factory(child);
        var obj = JObject.Parse(dep.ForFfi());
        var children = Assert.IsType<JArray>(obj["children"]);
        Assert.Single(children);
        Assert.Equal(expectedType, children[0]["type"]);
    }
}
