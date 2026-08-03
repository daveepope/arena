using System.Net;
using System.Threading.Tasks;
using ArenaExamples.Test.Shared;
using ArenaXunit;
using ArenaXunit.Playbook;
using ArenaXunit.Xunit;
using Xunit;

[assembly: PlaybookExecutionAttribute]

namespace ArenaExamples.ComponentTest;

public class ExampleComponentTests : IClassFixture<ExampleFixture>
{
    private static OpenArena Arena { get; set; } = null!;

    private readonly ApiClient api;

    public ExampleComponentTests(ExampleFixture fixture)
    {
        Arena = fixture.Arena;
        api = fixture.ApiClient;
    }

    [Fact]
    [Playbook(typeof(Playbooks.CalibrationOutagePlaybook))]
    [Playbook(typeof(Playbooks.ResetValidationDbPlaybook))]
    public async Task PostReadingReturns500WhenCalibrationOutageActive()
    {
        var response = await api.PostReadingRawAsync(new CreateReadingRequest { UserName = "Outage User", Value = 99, DeviceId = 1 });
        Assert.Equal(HttpStatusCode.InternalServerError, response.StatusCode);
    }

    [Fact]
    [Playbook(typeof(Playbooks.CalibrationOutagePlaybook))]
    [Playbook(typeof(Playbooks.ResetValidationDbPlaybook))]
    public async Task PostReadingReturns500UnderStackedPlaybooks()
    {
        var response = await api.PostReadingRawAsync(new CreateReadingRequest { UserName = "Stack Outage", Value = 1, DeviceId = 1 });
        Assert.Equal(HttpStatusCode.InternalServerError, response.StatusCode);
    }

    [Fact]
    public async Task GetDeviceStateUnknownDeviceReturnsNotFound()
    {
        var response = await api.GetDeviceStateRawAsync(999_999_999);
        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);
    }
}
