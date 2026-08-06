using System;
using System.Threading.Tasks;
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

    public DeviceWorkflowService(ITemporalClient client)
    {
        _client = client;
        _taskQueue = DeviceLifecycleWorkerHostedService.TaskQueue;
    }

    public async Task StartDeviceAsync(int deviceId)
    {
        var workflowId = $"device-{deviceId}";
        var options = new WorkflowOptions
        {
            Id = workflowId,
            TaskQueue = _taskQueue
        };
        await _client.StartWorkflowAsync((DeviceLifecycleWorkflow wf) => wf.Run(), options);
    }

    public async Task<bool> SignalTransitionAsync(int deviceId, string target)
    {
        var workflowId = $"device-{deviceId}";
        try
        {
            var handle = _client.GetWorkflowHandle<DeviceLifecycleWorkflow>(workflowId);
            await handle.SignalAsync((DeviceLifecycleWorkflow wf) => wf.RequestTransition(target));
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
            var handle = _client.GetWorkflowHandle<DeviceLifecycleWorkflow>(workflowId);
            var result = await handle.QueryAsync((DeviceLifecycleWorkflow wf) => wf.QuerySnapshot());
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
            var handle = _client.GetWorkflowHandle<DeviceLifecycleWorkflow>(workflowId);
            await handle.SignalAsync((DeviceLifecycleWorkflow wf) => wf.Stop());
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
