using System.Net;
using System.Threading.Tasks;
using ArenaExamples.Test.Shared;
using ArenaDotnet.Xunit;
using ArenaDotnet.Xunit.Playbook;
using ArenaDotnet.Xunit.Xunit;
using Xunit;

[assembly: PlaybookExecutionAttribute]

namespace ArenaExamples.ComponentTest;

public class ExampleComponentTests : IClassFixture<ExampleFixture>
{
    private static OpenArena Arena { get; set; } = null!;

    private readonly ApiClient api;
    private readonly ApiClient api2;

    public ExampleComponentTests(ExampleFixture fixture)
    {
        Arena = fixture.Arena;
        api = fixture.ApiClient;
        api2 = fixture.ApiClient2;
    }

    [Fact]
    [Playbook(typeof(Playbooks.ResetValidationDbPlaybook))]
    public async Task CreateDeviceRequestTransitionAppliesRequestedState()
    {
        var created = await api.CreateDeviceAsync(new CreateDeviceRequest { Name = "Smell-O-Scope Mk II" });

        var state = await api2.GetDeviceStateAsync(created.Id);
        Assert.Equal("OFF", state.State);

        await api2.SetDeviceStateAsync(created.Id, new SetDeviceStateRequest { Target = "ON" });
        state = await api.GetDeviceStateAsync(created.Id);
        Assert.Equal("ON", state.State);

        await api.SetDeviceStateAsync(created.Id, new SetDeviceStateRequest { Target = "ERROR" });
        state = await api2.GetDeviceStateAsync(created.Id);
        Assert.Equal("ERROR", state.State);

        await api2.StopDeviceAsync(created.Id);
    }
}
