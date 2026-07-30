namespace ArenaExamples.Readings.Aspnet.Workflows;

public class DeviceState
{
    public string CurrentState { get; set; } = "OFF";
    public int TransitionCount { get; set; }
}

[Temporal.Workflow]
public class DeviceLifecycleWorkflow
{
    private DeviceState _state = new DeviceState();

    public string QuerySnapshot()
    {
        return System.Text.Json.JsonSerializer.Serialize(_state);
    }

    public async Task RequestTransition(string target)
    {
        if (_state.CurrentState == target)
            return;

        await DeviceActivities.Transition(_state.CurrentState, target);
        _state.CurrentState = target;
        _state.TransitionCount++;
    }

    public async Task Stop()
    {
        await DeviceActivities.Stop(_state.CurrentState);
        throw new Temporal.WorkflowFailedException("Device stopped");
    }
}
