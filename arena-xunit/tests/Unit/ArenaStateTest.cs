using System;
using ArenaDotnet.Xunit.Lifecycle;
using Xunit;

namespace ArenaDotnet.Xunit.UnitTest;

public class ArenaStateTest
{
    internal const string FixtureStateJson =
        "{"
        + "\"id\":\"orders\",\"state\":\"arena_faulted\",\"at\":\"2026-09-06T11:02:44.812Z\","
        + "\"dependencies\":[{\"id\":\"orders-postgres\",\"state\":\"faulted\","
        + "\"faults\":[{\"id\":\"orders-postgres\",\"subject\":\"dependency\","
        + "\"message\":\"failed to start\",\"at\":\"2026-09-06T11:02:44.801Z\","
        + "\"faults\":[{\"id\":\"orders-postgres\",\"subject\":\"dependency\","
        + "\"message\":\"connection refused on 127.0.0.1:5432\","
        + "\"at\":\"2026-09-06T11:02:44.799Z\",\"faults\":[]}]}],"
        + "\"children\":[{\"id\":\"orders-postgres-seed\",\"state\":\"stopped\","
        + "\"faults\":[],\"children\":[]}]}],"
        + "\"components\":[{\"id\":\"orders-api\",\"state\":\"not_started\","
        + "\"faults\":[],\"children\":[]}],"
        + "\"faults\":[{\"id\":\"orders-postgres\",\"subject\":\"dependency\","
        + "\"message\":\"failed to start\",\"at\":\"2026-09-06T11:02:44.801Z\",\"faults\":[]}]"
        + "}";

    [Fact]
    public void Parse_FixtureDocument_RoundTripsEveryField()
    {
        var state = ArenaState.Parse(FixtureStateJson);

        Assert.Equal("orders", state.Id);
        Assert.Equal("arena_faulted", state.State);
        Assert.Equal("2026-09-06T11:02:44.812Z", state.At);
        Assert.Equal("orders-postgres", state.Dependencies[0].Id);
        Assert.Equal("faulted", state.Dependencies[0].State);
        Assert.Equal("orders-postgres-seed", state.Dependencies[0].Children[0].Id);
        Assert.Equal("not_started", state.Components[0].State);
        Assert.Equal("dependency", state.Faults[0].Subject);
        var cause = state.Dependencies[0].Faults[0].Faults[0];
        Assert.Equal("connection refused on 127.0.0.1:5432", cause.Message);
        Assert.Equal("2026-09-06T11:02:44.799Z", cause.At);
    }

    [Fact]
    public void Parse_NonObjectDocument_Throws()
    {
        Assert.Throws<ArgumentException>(() => ArenaState.Parse("[1, 2]"));
        Assert.Throws<ArgumentException>(() => ArenaState.Parse("{broken"));
        Assert.Throws<ArgumentException>(() => ArenaState.Parse("null"));
    }

    [Fact]
    public void Parse_MissingCollections_DefaultsToEmpty()
    {
        var state = ArenaState.Parse("{\"id\":\"bare\",\"state\":\"arena_created\",\"at\":\"t\"}");

        Assert.Empty(state.Dependencies);
        Assert.Empty(state.Components);
        Assert.Empty(state.Faults);
    }

    [Fact]
    public void Parse_UnknownField_IsTolerated()
    {
        var state = ArenaState.Parse("{\"id\":\"fwd\",\"state\":\"arena_open\",\"at\":\"t\",\"later\":1}");

        Assert.Equal("fwd", state.Id);
    }

    [Fact]
    public void IsFaulted_FaultedToken_ReturnsTrue()
    {
        Assert.True(ArenaState.Parse(FixtureStateJson).IsFaulted());
        Assert.False(ArenaState.Parse("{\"id\":\"x\",\"state\":\"arena_open\",\"at\":\"t\"}").IsFaulted());
    }

    [Fact]
    public void Dependency_NestedChildIdentifier_ReturnsThatChild()
    {
        var state = ArenaState.Parse(FixtureStateJson);

        var child = state.Dependency("orders-postgres-seed");
        Assert.NotNull(child);
        Assert.Equal("stopped", child!.State);
        Assert.Null(state.Dependency("no-such-dependency"));
    }

    [Fact]
    public void Component_NestedChildIdentifier_ReturnsThatChild()
    {
        var state = ArenaState.Parse(
            "{\"id\":\"n\",\"state\":\"arena_open\",\"at\":\"t\","
            + "\"components\":[{\"id\":\"parent\",\"state\":\"started\",\"faults\":[],"
            + "\"children\":[{\"id\":\"nested-child\",\"state\":\"started\",\"faults\":[],\"children\":[]}]}]}");

        var child = state.Component("nested-child");
        Assert.NotNull(child);
        Assert.Equal("started", child!.State);
        Assert.Null(state.Component("no-such-component"));
    }
}
