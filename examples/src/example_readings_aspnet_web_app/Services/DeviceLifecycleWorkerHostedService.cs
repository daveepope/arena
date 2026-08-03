using System.Threading;
using System.Threading.Tasks;
using ArenaExamples.Readings.Aspnet.Workflows;
using Microsoft.Extensions.Hosting;
using Temporalio.Client;
using Temporalio.Worker;

namespace ArenaExamples.Readings.Aspnet.Services;

public class DeviceLifecycleWorkerHostedService : BackgroundService
{
    public const string TaskQueue = "arena-example-device-lifecycle";

    private readonly ITemporalClient _client;

    public DeviceLifecycleWorkerHostedService(ITemporalClient client)
    {
        _client = client;
    }

    protected override async Task ExecuteAsync(CancellationToken stoppingToken)
    {
        var options = new TemporalWorkerOptions(TaskQueue)
            .AddWorkflow<DeviceLifecycleWorkflow>()
            .AddAllActivities(typeof(DeviceActivities), null);

        using var worker = new TemporalWorker(_client, options);
        await worker.ExecuteAsync(stoppingToken);
    }
}
