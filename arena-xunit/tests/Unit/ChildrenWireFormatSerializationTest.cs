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
    public void HttpDependency_WithChildDependencies_NestsChildConfig()
    {
        var child = new HttpDependencyBuilder("child").WithPort(9090).Build();
        var dep = new HttpDependencyBuilder("parent")
            .WithChildDependencies(new[] { child })
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
    public void ExecutableComponent_WithChildComponents_NestsChildConfig()
    {
        var child = new ExecutableComponentBuilder("child").WithExecutablePath("/bin/true").Build();
        var comp = new ExecutableComponentBuilder("parent")
            .WithExecutablePath("/bin/true")
            .WithChildComponents(new[] { child })
            .Build();
        var obj = JObject.Parse(comp.ForFfi());
        var children = Assert.IsType<JArray>(obj["children"]);
        Assert.Single(children);
        Assert.Equal("exec", children[0]["type"]);
    }

    public static IEnumerable<object[]> RemainingTypeFactories()
    {
        yield return new object[] { "kafka", (Func<string, IEnumerable<IArenaMatchPiece>, IArenaMatchPiece>)
            ((name, children) => new KafkaDependencyBuilder(name).WithChildDependencies(children).Build()) };
        yield return new object[] { "localstack", (Func<string, IEnumerable<IArenaMatchPiece>, IArenaMatchPiece>)
            ((name, children) => new LocalstackDependencyBuilder(name).WithChildDependencies(children).Build()) };
        yield return new object[] { "mssql", (Func<string, IEnumerable<IArenaMatchPiece>, IArenaMatchPiece>)
            ((name, children) => new MssqlDependencyBuilder(name).WithChildDependencies(children).Build()) };
        yield return new object[] { "oauth", (Func<string, IEnumerable<IArenaMatchPiece>, IArenaMatchPiece>)
            ((name, children) => new OauthDependencyBuilder(name).WithChildDependencies(children).Build()) };
        yield return new object[] { "postgres", (Func<string, IEnumerable<IArenaMatchPiece>, IArenaMatchPiece>)
            ((name, children) => new PostgresDependencyBuilder(name).WithChildDependencies(children).Build()) };
        yield return new object[] { "smtp", (Func<string, IEnumerable<IArenaMatchPiece>, IArenaMatchPiece>)
            ((name, children) => new SmtpDependencyBuilder(name).WithChildDependencies(children).Build()) };
        yield return new object[] { "temporal", (Func<string, IEnumerable<IArenaMatchPiece>, IArenaMatchPiece>)
            ((name, children) => new TemporalDependencyBuilder(name).WithChildDependencies(children).Build()) };
        yield return new object[] { "container", (Func<string, IEnumerable<IArenaMatchPiece>, IArenaMatchPiece>)
            ((name, children) => new ContainerizedComponentBuilder(name).WithContainerfile("Dockerfile").WithChildComponents(children).Build()) };
    }

    [Theory]
    [MemberData(nameof(RemainingTypeFactories))]
    public void DependencyOrComponent_NoChildren_OmitsChildrenKey(
        string expectedType, Func<string, IEnumerable<IArenaMatchPiece>, IArenaMatchPiece> factory)
    {
        var piece = factory("parent", Array.Empty<IArenaMatchPiece>());
        var obj = JObject.Parse(piece.ForFfi());
        Assert.Null(obj["children"]);
    }

    [Theory]
    [MemberData(nameof(RemainingTypeFactories))]
    public void DependencyOrComponent_WithChildren_NestsChildConfig(
        string expectedType, Func<string, IEnumerable<IArenaMatchPiece>, IArenaMatchPiece> factory)
    {
        var child = factory("child", Array.Empty<IArenaMatchPiece>());
        var piece = factory("parent", new[] { child });
        var obj = JObject.Parse(piece.ForFfi());
        var children = Assert.IsType<JArray>(obj["children"]);
        Assert.Single(children);
        Assert.Equal(expectedType, children[0]["type"]);
    }
}
