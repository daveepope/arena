using ArenaDotnet.Xunit.Component;
using Newtonsoft.Json.Linq;
using Xunit;

namespace ArenaDotnet.Xunit.UnitTest;

public class ExecutableComponentBuilderSerializationTest
{
    [Fact]
    public void Build_WithExecutablePath_SerializesCorrectJson()
    {
        var comp = new ExecutableComponentBuilder("test")
            .WithExecutablePath("./myapp")
            .Build();
        var json = comp.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("exec", obj["type"]);
        Assert.Equal("./myapp", obj["executable_path"]);
        Assert.NotNull(obj["identifier"]);
    }

    [Fact]
    public void Build_WithEnvVar_SerializesCorrectJson()
    {
        var comp = new ExecutableComponentBuilder("test")
            .WithExecutablePath("./myapp")
            .WithEnvVar("KEY", "value")
            .Build();
        var json = comp.ForFfi();
        var obj = JObject.Parse(json);
        Assert.NotNull(obj["env_vars"]);
        Assert.Equal("value", obj["env_vars"]["KEY"]);
    }

    [Fact]
    public void Build_WithRuntimeArg_SerializesCorrectJson()
    {
        var comp = new ExecutableComponentBuilder("test")
            .WithExecutablePath("./myapp")
            .WithRuntimeArg("web_app_port", "8080")
            .Build();
        var json = comp.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Single(obj["runtime_args"]);
        Assert.Equal("web_app_port", obj["runtime_args"][0]["name"]);
        Assert.Equal("8080", obj["runtime_args"][0]["value"]);
    }

    [Fact]
    public void Build_WithSourcePathAndBuildTool_SerializesCorrectJson()
    {
        var comp = new ExecutableComponentBuilder("test")
            .WithExecutablePath("./myapp")
            .WithSourcePath("./src")
            .WithBuildTool(BuildTool.Cargo)
            .Build();
        var json = comp.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("./src", obj["source_path"]);
        Assert.Equal("cargo", obj["build_tool"]);
    }

    [Fact]
    public void Build_WithCustomBuildTool_SerializesCommandAndArgs()
    {
        var comp = new ExecutableComponentBuilder("test")
            .WithExecutablePath("./myapp")
            .WithBuildTool(BuildTool.Custom("make", new[] { "release" }))
            .Build();
        var json = comp.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Equal("make", obj["build_tool"]["command"]);
        Assert.Equal("release", obj["build_tool"]["args"][0]);
    }

    [Fact]
    public void Build_WithReadinessCheck_SerializesHttpKind()
    {
        var comp = new ExecutableComponentBuilder("test")
            .WithExecutablePath("./myapp")
            .WithReadinessCheck(HttpReadinessCheck.Create(), "http://127.0.0.1:8080/health", 5000)
            .Build();
        var json = comp.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Single(obj["readiness_checks"]);
        Assert.Equal("http", obj["readiness_checks"][0]["kind"]);
        Assert.Equal("http://127.0.0.1:8080/health", obj["readiness_checks"][0]["target"]);
        Assert.Equal(5000, obj["readiness_checks"][0]["timeout_ms"]);
    }

    [Fact]
    public void Build_WithoutReadinessCheck_OmitsReadinessChecksField()
    {
        var comp = new ExecutableComponentBuilder("test")
            .WithExecutablePath("./myapp")
            .Build();
        var json = comp.ForFfi();
        var obj = JObject.Parse(json);
        Assert.Null(obj["readiness_checks"]);
    }

    [Fact]
    public void Build_WithoutExecutablePath_Throws()
    {
        var builder = new ExecutableComponentBuilder("test");
        Assert.Throws<System.InvalidOperationException>(() => builder.Build());
    }

    [Fact]
    public void Build_Identifier_MatchesPattern()
    {
        var comp = new ExecutableComponentBuilder("test")
            .WithExecutablePath("./myapp")
            .Build();
        Assert.StartsWith("arena-exec-", comp.Identifier);
    }
}
