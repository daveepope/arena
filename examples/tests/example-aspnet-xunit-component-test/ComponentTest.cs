using System;
using System.Collections.Generic;
using System.Linq;
using System.Net;
using System.Net.Http;
using System.Text.Json;
using System.Threading.Tasks;
using Amazon.SQS;
using Amazon.SQS.Model;
using ArenaExamples.Test.Shared;
using ArenaXunit;
using ArenaXunit.Dep;
using ArenaXunit.Topology;
using ArenaXunit.Playbook;
using ArenaXunit.Xunit;
using Xunit;

namespace ArenaExamples.ComponentTest;

public class AspNetComponentTests : IClassFixture<AspNetComponentTests.Fixture>, IDisposable
{
    private static int WebAppPort { get; } = EphemeralTestRuntime.AllocatePort();
    private static int PostgresPort { get; } = EphemeralTestRuntime.AllocatePort();
    private static int MssqlPort { get; } = EphemeralTestRuntime.AllocatePort();
    private static int CalibrationPort { get; } = EphemeralTestRuntime.AllocatePort();
    private static int LocalstackPort { get; } = EphemeralTestRuntime.AllocatePort();
    private static int TemporalPort { get; } = EphemeralTestRuntime.AllocatePort();
    private static int SmtpPort { get; } = EphemeralTestRuntime.AllocatePort();
    private static int SmtpUiPort { get; } = EphemeralTestRuntime.AllocatePort();
    private static int OauthPort { get; } = EphemeralTestRuntime.AllocatePort();

    private static string PostgresHost { get; } = "127.0.0.1";
    private static string MssqlHost { get; } = "127.0.0.1";

    private const string QueueName = "readings-events-queue";
    private const string EventBusName = "example-api-events";
    private const string EventSource = "readings.api";
    private const string EventRuleName = "readings-event-rule";
    private const string Region = "us-east-1";

    private readonly OpenArena arena;
    private readonly ApiClient api;
    private readonly string smtpUiBaseUrl;

    public AspNetComponentTests(AspNetComponentTests.Fixture collection)
    {
        this.arena = collection.Arena;
        this.api = collection.ApiClient;
        this.smtpUiBaseUrl = $"http://127.0.0.1:{SmtpUiPort}";
    }

    public void Dispose()
    {
    }

    public class TestTopology : IArenaTopology
    {
        public Match Configure()
        {
            Environment.SetEnvironmentVariable("WEB_APP_PORT", WebAppPort.ToString());
            Environment.SetEnvironmentVariable("CALIBRATION_URL", $"http://127.0.0.1:{CalibrationPort}");
            Environment.SetEnvironmentVariable("POSTGRES_HOST", PostgresHost);
            Environment.SetEnvironmentVariable("POSTGRES_PORT", PostgresPort.ToString());
            Environment.SetEnvironmentVariable("POSTGRES_DB", "testdb");
            Environment.SetEnvironmentVariable("POSTGRES_USER", "test");
            Environment.SetEnvironmentVariable("POSTGRES_PASSWORD", "test");
            Environment.SetEnvironmentVariable("POSTGRES_CONNECTION_STRING",
                $"host={PostgresHost} port={PostgresPort} user=test password=test dbname=testdb");
            Environment.SetEnvironmentVariable("MSSQL_HOST", MssqlHost);
            Environment.SetEnvironmentVariable("MSSQL_PORT", MssqlPort.ToString());
            Environment.SetEnvironmentVariable("MSSQL_DB_NAME", "testdb");
            Environment.SetEnvironmentVariable("MSSQL_DB_USER", "sa");
            Environment.SetEnvironmentVariable("MSSQL_DB_PASSWORD", "Password123!");
            Environment.SetEnvironmentVariable("MSSQL_CONNECTION_STRING",
                $"Server=tcp:{MssqlHost},{MssqlPort};Database=testdb;User Id=sa;Password=Password123!;TrustServerCertificate=True;");
            Environment.SetEnvironmentVariable("TEMPORAL_HOST", "127.0.0.1");
            Environment.SetEnvironmentVariable("TEMPORAL_PORT", TemporalPort.ToString());
            Environment.SetEnvironmentVariable("TEMPORAL_TARGET", $"127.0.0.1:{TemporalPort}");
            Environment.SetEnvironmentVariable("SMTP_HOST", "127.0.0.1");
            Environment.SetEnvironmentVariable("SMTP_PORT", SmtpPort.ToString());
            Environment.SetEnvironmentVariable("SMTP_FROM", "test@example.com");
            Environment.SetEnvironmentVariable("LOCALSTACK_HOST", $"127.0.0.1:{LocalstackPort}");
            Environment.SetEnvironmentVariable("LOCALSTACK_REGION", Region);
            Environment.SetEnvironmentVariable("AWS_REGION", Region);
            Environment.SetEnvironmentVariable("AWS_ENDPOINT_URL", $"http://127.0.0.1:{LocalstackPort}");
            Environment.SetEnvironmentVariable("AWS_DEFAULT_REGION", Region);
            Environment.SetEnvironmentVariable("AWS_ACCESS_KEY_ID", "test");
            Environment.SetEnvironmentVariable("AWS_SECRET_ACCESS_KEY", "test");
            Environment.SetEnvironmentVariable("EVENT_BUS_NAME", EventBusName);
            Environment.SetEnvironmentVariable("EVENT_SOURCE", EventSource);
            Environment.SetEnvironmentVariable("OAUTH_URL", $"http://127.0.0.1:{OauthPort}");
            Environment.SetEnvironmentVariable("OAUTH_ISSUER", $"http://127.0.0.1:{OauthPort}");
            Environment.SetEnvironmentVariable("OAUTH_CLIENT_ID", "test-client");
            Environment.SetEnvironmentVariable("OAUTH_CLIENT_SECRET", "test-secret");

            var eventRule = new EventRuleSpec
            {
                Name = EventRuleName,
                EventBus = EventBusName,
                EventPattern = JsonSerializer.Serialize(new { source = new[] { EventSource } }),
                Targets = new List<EventRuleTarget>
                {
                    EventRuleTargetBuilder.SqsQueue("target-queue", QueueName),
                },
            };

            var localstack = new LocalstackDependencyBuilder("test-localstack")
                .WithPort(LocalstackPort)
                .WithServices("sqs", "events")
                .WithQueue(QueueName)
                .WithEventBus(EventBusName)
                .WithEventRule(eventRule)
                .Build();

            var match = new MatchBuilder("aspnet")
                .AddDependency(new PostgresDependencyBuilder("test-postgres")
                    .WithPort(PostgresPort)
                    .Build())
                .AddDependency(new MssqlDependencyBuilder("test-mssql")
                    .WithPort(MssqlPort)
                    .Build())
                .AddDependency(localstack)
                .AddDependency(new TemporalDependencyBuilder("test-temporal")
                    .WithPort(TemporalPort)
                    .Build())
                .AddDependency(new OauthDependencyBuilder("test-oauth")
                    .WithPort(OauthPort)
                    .Build())
                .AddDependency(new SmtpDependencyBuilder("test-smtp")
                    .WithPort(SmtpPort)
                    .WithUiPort(SmtpUiPort)
                    .WithStarttls()
                    .Build())
                .AddDependency(new HttpDependencyBuilder("test-calibration")
                    .WithPort(CalibrationPort)
                    .Build())
                .RegisterPlaybook(new Playbooks.CalibrationHappyPathPlaybook("test-calibration"), true)
                .RegisterPlaybook(new Playbooks.CalibrationOutagePlaybook("test-calibration"), false)
                .RegisterPlaybook(new Playbooks.CalibrationFlakyPlaybook("test-calibration"), false)
                .RegisterPlaybook(new Playbooks.ResetValidationDbPlaybook("test-mssql"), false)
                .RegisterPlaybook(new Playbooks.EventsPurgePlaybook("test-localstack"), true)
                .Build();
            return match;
        }
    }

    public class Fixture : ArenaCollectionFixture<TestTopology>
    {
        public ApiClient ApiClient { get; }

        public Fixture() : base()
        {
            var authToken = GetAuthToken();
            ApiClient = new ApiClient($"http://127.0.0.1:{WebAppPort}", authToken);
        }

        private static string GetAuthToken()
        {
            using var client = new HttpClient();
            var content = new Dictionary<string, string>
            {
                ["grant_type"] = "client_credentials",
                ["client_id"] = "test-client",
                ["client_secret"] = "test-secret"
            };
            var response = client.PostAsync($"http://127.0.0.1:{OauthPort}/oauth/token",
                new FormUrlEncodedContent(content)).Result;
            response.EnsureSuccessStatusCode();
            var json = JsonSerializer.Deserialize<JsonElement>(response.Content.ReadAsStringAsync().Result);
            return json.GetProperty("access_token").GetString();
        }
    }

    private async Task<JsonElement> WaitReadingCreatedEventAsync(int expectedId, int maxAttempts = 30)
    {
        var endpoint = new Uri($"http://127.0.0.1:{LocalstackPort}");
        var client = new AmazonSQSClient(new AmazonSQSConfig { ServiceURL = endpoint.ToString() });
        var queueUrl = (await client.GetQueueUrlAsync(QueueName)).QueueUrl;

        for (int i = 0; i < maxAttempts; i++)
        {
            var resp = await client.ReceiveMessageAsync(new ReceiveMessageRequest
            {
                QueueUrl = queueUrl,
                MaxNumberOfMessages = 1,
                WaitTimeSeconds = 2,
                VisibilityTimeout = 10,
            }).ConfigureAwait(false);

            foreach (var msg in resp.Messages)
            {
                var body = JsonSerializer.Deserialize<JsonElement>(msg.Body);
                var detailType = body.TryGetProperty("detail-type", out var dt) ? dt.GetString() : null;
                if (detailType != "ReadingCreated")
                {
                    await client.DeleteMessageAsync(queueUrl, msg.ReceiptHandle).ConfigureAwait(false);
                    continue;
                }

                var detail = body.TryGetProperty("detail", out var d) ? d : JsonDocument.Parse("{}").RootElement;
                var id = detail.TryGetProperty("id", out var idProp) ? idProp.GetInt32() : -1;

                await client.DeleteMessageAsync(queueUrl, msg.ReceiptHandle).ConfigureAwait(false);

                if (id == expectedId)
                {
                    return detail;
                }
            }

            await Task.Delay(100).ConfigureAwait(false);
        }

        throw new TimeoutException($"SQS did not receive ReadingCreated for id={expectedId}");
    }

    private async Task<bool> WaitDeviceProvisionedEmailAsync(string deviceName, int maxAttempts = 30)
    {
        using var client = new HttpClient();
        for (int i = 0; i < maxAttempts; i++)
        {
            try
            {
                var response = await client.GetAsync($"{smtpUiBaseUrl}/api/v1/messages").ConfigureAwait(false);
                if (response.IsSuccessStatusCode)
                {
                    var text = await response.Content.ReadAsStringAsync().ConfigureAwait(false);
                    if (text.Contains(deviceName))
                    {
                        return true;
                    }
                }
            }
            catch
            {
            }
            await Task.Delay(100).ConfigureAwait(false);
        }
        return false;
    }

    [Fact]
    public async Task CreateReadingPublishesEventAndListsViaHttp()
    {
        using var pb = arena.GetPlaybook<Playbooks.EventsPurgePlaybook>().Run(arena);
        var reading = new CreateReadingRequest
        {
            UserName = "Test User",
            Value = 77,
            Comment = "sqs happy path",
            DeviceId = 1,
        };
        var created = await api.CreateReadingAsync(reading);
        Assert.True(created.Id > 0);

        var consumed = await WaitReadingCreatedEventAsync(created.Id);
        Assert.Equal(created.Id, consumed.TryGetProperty("id", out var id) ? id.GetInt32() : -1);
        Assert.Equal("Test User", consumed.TryGetProperty("user_name", out var un) ? un.GetString() : "");
        Assert.Equal(77, consumed.TryGetProperty("value", out var v) ? v.GetInt32() : -1);
        Assert.Equal("sqs happy path", consumed.TryGetProperty("comment", out var c) ? c.GetString() : "");

        var listed = await api.ListReadingsAsync();
        Assert.Contains(listed, r => r.Id == created.Id && r.Value == 77);
    }

    [Fact]
    public async Task CreateMultipleReadingsReturnsAllReadings()
    {
        await api.CreateReadingAsync(new CreateReadingRequest { UserName = "User1", Value = 1, DeviceId = 1 });
        await api.CreateReadingAsync(new CreateReadingRequest { UserName = "User2", Value = 2, DeviceId = 1 });
        var listed = await api.ListReadingsAsync();
        Assert.True(listed.Count >= 2);
    }

    [Fact]
    public async Task PostReadingReturns500WhenCalibrationOutageActive()
    {
        using var pb = arena.GetPlaybook<Playbooks.CalibrationOutagePlaybook>().Run(arena);
        var response = await api.PostReadingRawAsync(new CreateReadingRequest { UserName = "Outage User", Value = 99, DeviceId = 1 });
        Assert.Equal(HttpStatusCode.InternalServerError, response.StatusCode);
    }

    [Fact]
    public async Task PostReadingSucceedsAfterOutagePlaybookScope()
    {
        {
            using var pb = arena.GetPlaybook<Playbooks.CalibrationOutagePlaybook>().Run(arena);
            var response = await api.PostReadingRawAsync(new CreateReadingRequest { UserName = "Outage", Value = 1, DeviceId = 1 });
            Assert.Equal(HttpStatusCode.InternalServerError, response.StatusCode);
        }
        var ok = await api.CreateReadingAsync(new CreateReadingRequest { UserName = "Recovery", Value = 17, DeviceId = 1 });
        Assert.True(ok.Id > 0);
    }

    [Fact]
    public async Task CreateReadingWithValidationDbScopedPlaybook()
    {
        using var pb = arena.GetPlaybook<Playbooks.ResetValidationDbPlaybook>().Run(arena);
        var reading = new CreateReadingRequest { UserName = "Scoped User", Value = 7, DeviceId = 2 };
        var created = await api.CreateReadingAsync(reading);
        Assert.True(created.Id > 0);
        var listed = await api.ListReadingsAsync();
        Assert.Contains(listed, r => r.Id == created.Id);
    }

    [Fact]
    public async Task PostReadingReturns500UnderStackedPlaybooks()
    {
        using var pb1 = arena.GetPlaybook<Playbooks.CalibrationOutagePlaybook>().Run(arena);
        using var pb2 = arena.GetPlaybook<Playbooks.ResetValidationDbPlaybook>().Run(arena);
        var response = await api.PostReadingRawAsync(new CreateReadingRequest { UserName = "Stack Outage", Value = 1, DeviceId = 1 });
        Assert.Equal(HttpStatusCode.InternalServerError, response.StatusCode);
    }

    [Fact]
    public async Task PostReadingSucceedsAfterCalibrationFlakySequence()
    {
        using var pb = arena.GetPlaybook<Playbooks.CalibrationFlakyPlaybook>().Run(arena);
        Assert.Equal(HttpStatusCode.InternalServerError,
            (await api.PostReadingRawAsync(new CreateReadingRequest { UserName = "Flaky1", Value = 1, DeviceId = 1 })).StatusCode);
        Assert.Equal(HttpStatusCode.InternalServerError,
            (await api.PostReadingRawAsync(new CreateReadingRequest { UserName = "Flaky2", Value = 2, DeviceId = 1 })).StatusCode);
        var ok = await api.CreateReadingAsync(new CreateReadingRequest { UserName = "Flaky3", Value = 3, DeviceId = 1 });
        Assert.True(ok.Id > 0);
    }

    [Fact]
    public async Task SetDeviceStateAppliesRequestedState()
    {
        var created = await api.CreateDeviceAsync(new CreateDeviceRequest { Name = "Test Device" });
        Assert.True(created.Id > 0);

        var state = await api.GetDeviceStateAsync(created.Id);
        Assert.Equal("OFF", state.State);

        await api.SetDeviceStateAsync(created.Id, new SetDeviceStateRequest { Target = "ON" });
        state = await api.GetDeviceStateAsync(created.Id);
        Assert.Equal("ON", state.State);

        await api.SetDeviceStateAsync(created.Id, new SetDeviceStateRequest { Target = "ERROR" });
        state = await api.GetDeviceStateAsync(created.Id);
        Assert.Equal("ERROR", state.State);

        var stopped = await api.StopDeviceAsync(created.Id);
        Assert.True(stopped);
    }

    [Fact]
    public async Task GetDeviceStateUnknownDeviceReturnsNotFound()
    {
        var response = await api.GetDeviceStateRawAsync(999_999_999);
        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);
    }

    [Fact]
    public async Task CreateDeviceSendsProvisionedEmail()
    {
        var deviceName = $"Email Test Device {Guid.NewGuid():N}";
        var created = await api.CreateDeviceAsync(new CreateDeviceRequest { Name = deviceName });
        Assert.True(created.Id > 0);

        var emailFound = await WaitDeviceProvisionedEmailAsync(deviceName);
        Assert.True(emailFound, $"Provisioned email for {deviceName} was not captured");
    }
}
