using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Net;
using System.Net.Http;
using System.Text.Json;
using System.Threading.Tasks;
using Amazon.SQS;
using Amazon.SQS.Model;
using ArenaExamples.Test.Shared;
using ArenaXunit;
using ArenaXunit.Component;
using ArenaXunit.Dep;
using ArenaXunit.Playbook;
using ArenaXunit.Xunit;
using ArenaXunit.Topology;
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
    private static int OauthPort { get; } = EphemeralTestRuntime.AllocatePort();

    private readonly OpenArena arena;
    private readonly ApiClient api;

    private static string PostgresHost { get; } = "127.0.0.1";
    private static string MssqlHost { get; } = "127.0.0.1";



    public AspNetComponentTests(AspNetComponentTests.Fixture collection)
    {
        this.arena = collection.Arena;
        this.api = collection.ApiClient;
    }

    public void Dispose()
    {
    }

    public class TestTopology : IArenaTopology
    {
        public Match Configure()
        {
            Environment.SetEnvironmentVariable("WEB_APP_PORT", WebAppPort.ToString());
            Environment.SetEnvironmentVariable("CALIBRATION_URL", $"http://127.0.0.1:{CalibrationPort}/api/v1/calibrate");
            Environment.SetEnvironmentVariable("LOCALSTACK_HOST", $"127.0.0.1:{LocalstackPort}");
            Environment.SetEnvironmentVariable("LOCALSTACK_REGION", "us-east-1");
            Environment.SetEnvironmentVariable("AWS_REGION", "us-east-1");
            Environment.SetEnvironmentVariable("LOCALSTACK_SQS_ENDPOINT", $"http://127.0.0.1:{LocalstackPort}");
            Environment.SetEnvironmentVariable("SMTP_HOST", "127.0.0.1");
            Environment.SetEnvironmentVariable("SMTP_PORT", SmtpPort.ToString());
            Environment.SetEnvironmentVariable("SMTP_FROM", "test@example.com");
            Environment.SetEnvironmentVariable("TEMPORAL_HOST", "127.0.0.1");
            Environment.SetEnvironmentVariable("TEMPORAL_PORT", TemporalPort.ToString());
            Environment.SetEnvironmentVariable("JWT_ISSUER", "test-issuer");
            Environment.SetEnvironmentVariable("JWT_AUDIENCE", "test-audience");
            Environment.SetEnvironmentVariable("JWT_KEY", "super-secret-key-that-is-long-enough-for-jwt-signing-purposes");
            Environment.SetEnvironmentVariable("VALIDATION_DB_HOST", MssqlHost);
            Environment.SetEnvironmentVariable("VALIDATION_DB_PORT", MssqlPort.ToString());
            Environment.SetEnvironmentVariable("VALIDATION_DB_USER", "sa");
            Environment.SetEnvironmentVariable("VALIDATION_DB_PASSWORD", "Password123!");
            Environment.SetEnvironmentVariable("VALIDATION_DB_NAME", "testdb");
            Environment.SetEnvironmentVariable("POSTGRES_HOST", PostgresHost);
            Environment.SetEnvironmentVariable("POSTGRES_PORT", PostgresPort.ToString());
            Environment.SetEnvironmentVariable("POSTGRES_DB", "testdb");
            Environment.SetEnvironmentVariable("POSTGRES_USER", "test");
            Environment.SetEnvironmentVariable("POSTGRES_PASSWORD", "test");
            Environment.SetEnvironmentVariable("OAUTH_URL", $"http://127.0.0.1:{OauthPort}");
            Environment.SetEnvironmentVariable("OAUTH_CLIENT_ID", "test-client");
            Environment.SetEnvironmentVariable("OAUTH_CLIENT_SECRET", "test-secret");

            var match = new MatchBuilder("aspnet")
                .AddDependency(new PostgresDependencyBuilder("test-postgres")
                    .WithPort(PostgresPort)
                    .Build())
                .AddDependency(new MssqlDependencyBuilder("test-mssql")
                    .WithPort(MssqlPort)
                    .Build())
                .AddDependency(new LocalstackDependencyBuilder("test-localstack")
                    .WithPort(LocalstackPort)
                    .Build())
                .AddDependency(new TemporalDependencyBuilder("test-temporal")
                    .WithPort(TemporalPort)
                    .Build())
                .AddDependency(new OauthDependencyBuilder("test-oauth")
                    .WithPort(OauthPort)
                    .Build())
                .AddDependency(new SmtpDependencyBuilder("test-smtp")
                    .WithPort(SmtpPort)
                    .Build())
                .AddDependency(new HttpDependencyBuilder("test-calibration")
                    .WithPort(CalibrationPort)
                    .Build())
                .AddComponent(new ExecutableComponentBuilder("test-webapp")
                    .WithExecutablePath("/path/to/app")
                    .WithEnv("DOTNET_RUNNING_IN_TESTS", "1")
                    .Build())
                .RegisterPlaybook(new Playbooks.CalibrationHappyPathPlaybook("test-calibration"), false)
                .RegisterPlaybook(new Playbooks.CalibrationOutagePlaybook("test-calibration"), false)
                .RegisterPlaybook(new Playbooks.CalibrationFlakyPlaybook("test-calibration"), false)
                .RegisterPlaybook(new Playbooks.ResetValidationDbPlaybook("test-mssql"), false)
                .RegisterPlaybook(new Playbooks.EventsPurgePlaybook("test-localstack"), false)
                .RegisterPlaybook(new Playbooks.TrafficVerifyAtLeast("test-calibration"), false)
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

    private async Task PollSqsForEventAsync(string topic, int maxAttempts = 30)
    {
        var endpoint = new Uri($"http://127.0.0.1:{LocalstackPort}");
        var client = new AmazonSQSClient(new AmazonSQSConfig { ServiceURL = endpoint.ToString() });
        var queueName = $"{topic}.queue";
        try { await client.DeleteQueueAsync(queueName).ConfigureAwait(false); } catch { }
        await client.CreateQueueAsync(queueName);
        await client.SetQueueAttributesAsync(queueName, new Dictionary<string, string>
        {
            ["RedrivePolicy"] = "{}"
        });
        var attrs = (await client.GetQueueAttributesAsync(new GetQueueAttributesRequest
        {
            QueueUrl = await GetQueueUrl(client, queueName),
            AttributeNames = new List<string> { "QueueArn" }
        })).Attributes["QueueArn"];

        // Use SNS to send message to queue (simulating topic subscription)
        for (int i = 0; i < maxAttempts; i++)
        {
            var resp = await client.ReceiveMessageAsync(new ReceiveMessageRequest
            {
                QueueUrl = await GetQueueUrl(client, queueName),
                MaxNumberOfMessages = 1,
                WaitTimeSeconds = 1
            }).ConfigureAwait(false);
            if (resp.Messages.Count > 0) return;
        }
        throw new TimeoutException($"No messages received from {queueName} within timeout");
    }

    private async Task<string> GetQueueUrl(AmazonSQSClient client, string queueName)
    {
        var resp = await client.GetQueueUrlAsync(queueName).ConfigureAwait(false);
        return resp.QueueUrl;
    }

    private async Task<string> PollSmtpForEmailAsync(string recipient, int maxAttempts = 30)
    {
        for (int i = 0; i < maxAttempts; i++)
        {
            try
            {
                var response = await api.GetRawAsync($"/api/smtp-mailbox/{Uri.EscapeDataString(recipient)}").ConfigureAwait(false);
                if (response.IsSuccessStatusCode)
                {
                    var json = await response.Content.ReadAsStringAsync().ConfigureAwait(false);
                    var emails = JsonSerializer.Deserialize<List<JsonElement>>(json);
                    if (emails != null && emails.Count > 0)
                        return emails[0].GetProperty("Body").GetString();
                }
            }
            catch { }
            await Task.Delay(500).ConfigureAwait(false);
        }
        throw new TimeoutException($"No email received for {recipient}");
    }

    [Fact]
    public async Task createReading_publishesEventAndListsViaHttp()
    {
        using var pb = arena.GetPlaybook(typeof(Playbooks.EventsPurgePlaybook)).Run(arena);
        var reading = new CreateReadingRequest
        {
            DeviceId = "device-1",
            TemperatureC = 21.5
        };
        var created = await api.CreateReadingAsync(reading);
        Assert.NotNull(created.Id);

        await PollSqsForEventAsync("readings-topic");

        var listed = await api.ListReadingsAsync("device-1");
        Assert.Single(listed);
        Assert.Equal("device-1", listed[0].DeviceId);
    }

    [Fact]
    public async Task createMultipleReadings_areListed()
    {
        for (int i = 0; i < 3; i++)
        {
            await api.CreateReadingAsync(new CreateReadingRequest
            {
                DeviceId = "device-1",
                TemperatureC = 20.0 + i
            });
        }
        var listed = await api.ListReadingsAsync("device-1");
        Assert.Equal(3, listed.Count);
    }

    [Fact]
    public async Task postReading_returns500_whenCalibrationOutage_active()
    {
        using var pb = arena.GetPlaybook(typeof(Playbooks.CalibrationOutagePlaybook)).Run(arena);
        var reading = new CreateReadingRequest { DeviceId = "device-1", TemperatureC = 21.0 };
        var response = await api.PostReadingRawAsync(reading);
        Assert.Equal(HttpStatusCode.InternalServerError, response.StatusCode);
    }

    [Fact]
    public async Task postReading_succeedsAfterOutage_playbookIsolation()
    {
        {
            using var pb = arena.GetPlaybook(typeof(Playbooks.CalibrationOutagePlaybook)).Run(arena);
            var response = await api.PostReadingRawAsync(new CreateReadingRequest { DeviceId = "d", TemperatureC = 20.0 });
            Assert.Equal(HttpStatusCode.InternalServerError, response.StatusCode);
        }
        var ok = await api.CreateReadingAsync(new CreateReadingRequest { DeviceId = "d", TemperatureC = 20.0 });
        Assert.NotNull(ok.Id);
    }

    [Fact]
    public async Task createReading_withValidationDbScopedPlaybook()
    {
        using var pb = arena.GetPlaybook(typeof(Playbooks.ResetValidationDbPlaybook)).Run(arena);
        var reading = new CreateReadingRequest { DeviceId = "device-2", TemperatureC = 22.0 };
        var created = await api.CreateReadingAsync(reading);
        Assert.NotNull(created.Id);
        Assert.Equal("device-2", created.DeviceId);
    }

    [Fact]
    public async Task postReading_returns500_underStackedPlaybooks()
    {
        using var pb1 = arena.GetPlaybook(typeof(Playbooks.CalibrationOutagePlaybook)).Run(arena);
        using var pb2 = arena.GetPlaybook(typeof(Playbooks.ResetValidationDbPlaybook)).Run(arena);
        var response = await api.PostReadingRawAsync(new CreateReadingRequest { DeviceId = "d", TemperatureC = 20.0 });
        Assert.Equal(HttpStatusCode.InternalServerError, response.StatusCode);
    }

    [Fact]
    public async Task postReading_succeedsAfterCalibrationFlakySequence()
    {
        using var pb = arena.GetPlaybook(typeof(Playbooks.CalibrationFlakyPlaybook)).Run(arena);
        for (int attempt = 0; attempt < 3; attempt++)
        {
            try
            {
                var created = await api.CreateReadingAsync(new CreateReadingRequest { DeviceId = "d", TemperatureC = 20.0 });
                Assert.NotNull(created.Id);
                return;
            }
            catch (Exception ex) when (ex is TaskCanceledException || ex.Message.Contains("503") || ex.Message.Contains("500"))
            {
            }
        }
        throw new InvalidOperationException("Reading creation did not succeed within allowed attempts under flaky calibration");
    }

    [Fact]
    public async Task httpPlaybook_verifyAtLeast_succeedsWithTraffic()
    {
        using var pb = arena.GetPlaybook(typeof(Playbooks.TrafficVerifyAtLeast)).Run(arena);
        await api.GetRawAsync("/api/health").ConfigureAwait(false);
        await api.GetRawAsync("/api/health").ConfigureAwait(false);
        await pb.VerifyAtLeast("/api/v1/calibrate", 1);
    }

    [Fact]
    public async Task httpPlaybook_verifyCountMismatch_raises()
    {
        using var pb = arena.GetPlaybook(typeof(Playbooks.TrafficVerifyAtLeast)).Run(arena);
        await Assert.ThrowsAsync<Exception>(async () => await pb.VerifyAtLeast("/api/v1/calibrate", 100));
    }

    [Fact]
    public async Task createDevice_requestTransition_appliesState()
    {
        var device = new CreateDeviceRequest
        {
            Name = "sensor-1",
            Location = "room-A",
            Type = "temperature",
            TargetState = "ACTIVE"
        };
        var created = await api.CreateDeviceAsync(device);
        Assert.Equal("PENDING", created.State);

        await api.RequestStateTransitionAsync(created.Id, new DeviceStateTransitionRequest { TargetState = "ACTIVE" });
        var fetched = await api.GetDeviceAsync(created.Id);
        Assert.Equal("ACTIVE", fetched.State);
    }

    [Fact]
    public async Task getDeviceState_unknownDevice_returnsNotFound()
    {
        var response = await api.GetDeviceRawAsync("nonexistent-device-id");
        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);
    }

    [Fact]
    public async Task createDevice_sendsProvisionedEmail_overStarttls()
    {
        var device = new CreateDeviceRequest
        {
            Name = "email-test-device",
            Location = "room-B",
            Type = "temperature",
            TargetState = "ACTIVE"
        };
        await api.CreateDeviceAsync(device);
        await Task.Delay(500).ConfigureAwait(false);

        var emailBody = await PollSmtpForEmailAsync("admin@example.com");
        Assert.Contains("Provisioned", emailBody);
        Assert.Contains("email-test-device", emailBody);
    }
}


