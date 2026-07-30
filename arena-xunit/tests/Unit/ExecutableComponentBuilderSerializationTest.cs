using ArenaXunit.Component;
using Newtonsoft.Json.Linq;
using Xunit;

namespace ArenaXunit.UnitTest;

public class ExecutableComponentBuilderSerializationTest
{
    [Fact]
    public void build_with_executable_path_serializes_correct_json()
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
    public void build_with_env_serializes_correct_json()
    {
        var comp = new ExecutableComponentBuilder("test")
            .WithExecutablePath("./myapp")
            .WithEnv("KEY", "value")
            .Build();
        var json = comp.ForFfi();
        var obj = JObject.Parse(json);
        Assert.NotNull(obj["env"]);
        Assert.Equal("value", obj["env"]["key"]);
    }

    [Fact]
    public void build_with_args_serializes_correct_json()
    {
        var comp = new ExecutableComponentBuilder("test")
            .WithExecutablePath("./myapp")
            .WithArgs("--flag", "arg1")
            .Build();
        var json = comp.ForFfi();
        var obj = JObject.Parse(json);
        Assert.NotNull(obj["args"]);
        Assert.Equal("--flag", obj["args"][0]);
        Assert.Equal("arg1", obj["args"][1]);
    }

    [Fact]
    public void build_without_executable_path_throws()
    {
        var builder = new ExecutableComponentBuilder("test");
        Assert.Throws<System.InvalidOperationException>(() => builder.Build());
    }

    [Fact]
    public void build_identifier_matches_pattern()
    {
        var comp = new ExecutableComponentBuilder("test")
            .WithExecutablePath("./myapp")
            .Build();
        Assert.StartsWith("arena-exec-", comp.Identifier);
    }
}
