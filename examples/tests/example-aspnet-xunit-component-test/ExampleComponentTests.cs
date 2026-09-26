using System;
using System.Net;
using System.Net.Http;
using System.Net.Http.Headers;
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

    private readonly ExampleFixture fixture;
    private readonly ApiClient api;
    private readonly ApiClient api2;

    public ExampleComponentTests(ExampleFixture fixture)
    {
        this.fixture = fixture;
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

    [Fact]
    [Playbook(typeof(Playbooks.ResetWeatherDbPlaybook))]
    public async Task CreateWeatherReportListsViaHttp()
    {
        var created = await api.CreateWeatherReportAsync(new CreateWeatherReportRequest
        {
            Precipitation = 1.5,
            Humidity = 63.2,
            Pressure = 1013.25,
        });

        var reports = await api.ListWeatherReportsAsync();
        var found = reports.Find(r => r.Id == created.Id);
        Assert.NotNull(found);
        Assert.Equal(1.5, found.Precipitation);
        Assert.Equal(63.2, found.Humidity);
        Assert.Equal(1013.25, found.Pressure);
    }

    [Fact]
    [Playbook(typeof(Playbooks.ResetWeatherDbPlaybook))]
    public async Task CreateMultipleWeatherReportsAreListed()
    {
        var created1 = await api.CreateWeatherReportAsync(new CreateWeatherReportRequest
        {
            Precipitation = 0,
            Humidity = 40,
            Pressure = 1000,
        });
        var created2 = await api.CreateWeatherReportAsync(new CreateWeatherReportRequest
        {
            Precipitation = 2.2,
            Humidity = 80,
            Pressure = 990.5,
        });

        var reports = await api.ListWeatherReportsAsync();
        Assert.Contains(reports, r => r.Id == created1.Id);
        Assert.Contains(reports, r => r.Id == created2.Id);
    }

    [Fact]
    public async Task GetReadingsWithoutBearerToken_IsRejected()
    {
        using var client = new HttpClient { Timeout = TimeSpan.FromSeconds(10) };
        var response = await client.GetAsync($"http://127.0.0.1:{ExampleFixture.WebAppPort}/Readings");
        Assert.Equal(
            HttpStatusCode.Unauthorized,
            response.StatusCode);
    }

    [Fact]
    public async Task GetReadingsWithTokenMissingRequiredScope_IsRejected()
    {
        var token = fixture.Signer.Sign(ExampleFixture.OauthProvider(), ExampleFixture.ClaimsWithScope("other-scope"));
        using var client = new HttpClient { Timeout = TimeSpan.FromSeconds(10) };
        client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue("Bearer", token);
        var response = await client.GetAsync($"http://127.0.0.1:{ExampleFixture.WebAppPort}/Readings");
        Assert.Equal(
            HttpStatusCode.Forbidden,
            response.StatusCode);
    }
}
