using ArenaExamples.Readings.Aspnet.Models;
using ArenaExamples.Readings.Aspnet.Workflows;
using Temporalio.Client;

namespace ArenaExamples.Readings.Aspnet.Services;

public interface IDeviceWorkflowService
{
    Task StartDeviceAsync(int deviceId);
    Task<bool> SignalTransitionAsync(int deviceId, string target);
    Task<DeviceStateResponse?> GetStateAsync(int deviceId);
    Task<bool> StopDeviceAsync(int deviceId);
}

public class DeviceWorkflowService : IDeviceWorkflowService
{
    private readonly ITemporalClient _client;
    private readonly string _taskQueue;

    public DeviceWorkflowService(TemporalClient client)
    {
        _client = client;
        _taskQueue = "arena-example-device-lifecycle";
    }

    public async Task StartDeviceAsync(int deviceId)
    {
        var workflowId = $"device-{deviceId}";
        await _client.StartWorkflowAsync(new WorkflowStartOptions<DeviceLifecycleWorkflow, DeviceStateResponse?>
        {
            Id = workflowId,
            TaskQueue = _taskQueue
        });
    }

    public async Task<bool> SignalTransitionAsync(int deviceId, string target)
    {
        var workflowId = $"device-{deviceId}";
        try
        {
            await _client.SignalWorkflowAsync(workflowId, nameof(DeviceLifecycleWorkflow.RequestTransition), target);
            return true;
        }
        catch
        {
            return false;
        }
    }

    public async Task<DeviceStateResponse?> GetStateAsync(int deviceId)
    {
        var workflowId = $"device-{deviceId}";
        try
        {
            var handle = _client.GetWorkflowHandle(workflowId);
            var result = await handle.QueryAsync(nameof(DeviceLifecycleWorkflow.QuerySnapshot));
            var state = System.Text.Json.JsonSerializer.Deserialize<DeviceState>(result);
            if (state == null)
                return null;
            return new DeviceStateResponse
            {
                DeviceId = deviceId,
                State = state.CurrentState,
                TransitionCount = state.TransitionCount
            };
        }
        catch
        {
            return null;
        }
    }

    public async Task<bool> StopDeviceAsync(int deviceId)
    {
        var workflowId = $"device-{deviceId}";
        try
        {
            await _client.SignalWorkflowAsync(workflowId, nameof(DeviceLifecycleWorkflow.Stop));
            return true;
        }
        catch
        {
            return false;
        }
    }

    private class DeviceState
    {
        public string CurrentState { get; set; } = "OFF";
        public int TransitionCount { get; set; }
    }
}
