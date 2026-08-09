using System.Threading;
using System.Threading.Tasks;
using ArenaExamples.Readings.Aspnet.Workflows;
using Microsoft.Extensions.Hosting;
using Temporalio.Api.Enums.V1;
using Temporalio.Api.WorkflowService.V1;
using Temporalio.Client;
using Temporalio.Worker;

namespace ArenaExamples.Readings.Aspnet.Services;

public class DeviceLifecycleWorkerHostedService : BackgroundService
{
    public const string TaskQueue = "arena-example-device-lifecycle";

    private readonly ITemporalClient _client;

    public bool IsReady { get; private set; }

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
        var workerTask = worker.ExecuteAsync(stoppingToken);
        await WaitForPollerAsync(stoppingToken);
        IsReady = true;
        await workerTask;
    }

    private async Task WaitForPollerAsync(CancellationToken cancellationToken)
    {
        while (!cancellationToken.IsCancellationRequested)
        {
            var response = await _client.WorkflowService.DescribeTaskQueueAsync(new DescribeTaskQueueRequest
            {
                Namespace = "default",
                TaskQueue = new Temporalio.Api.TaskQueue.V1.TaskQueue { Name = TaskQueue },
                TaskQueueType = TaskQueueType.Workflow,
            });
            if (response.Pollers.Count > 0)
                return;
            await Task.Delay(50, cancellationToken);
        }
    }
}
