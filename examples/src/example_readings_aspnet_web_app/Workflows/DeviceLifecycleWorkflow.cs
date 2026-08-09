using System;
using System.Threading.Tasks;
using Temporalio.Workflows;
using Temporalio.Exceptions;
namespace ArenaExamples.Readings.Aspnet.Workflows;

[Workflow]
public class DeviceLifecycleWorkflow
{
    private string _currentState = "OFF";
    private int _transitionCount;

    [WorkflowRun]
    public async Task Run()
    {
        while (true)
        {
            await Workflow.DelayAsync(TimeSpan.FromSeconds(1));
        }
    }

    [WorkflowQuery]
    public string QuerySnapshot()
    {
        return System.Text.Json.JsonSerializer.Serialize(new
        {
            CurrentState = _currentState,
            TransitionCount = _transitionCount
        });
    }

    private static readonly ActivityOptions TransitionActivityOptions = new()
    {
        StartToCloseTimeout = TimeSpan.FromSeconds(10),
    };

    [WorkflowSignal]
    public async Task RequestTransition(string target)
    {
        if (_currentState == target)
            return;

        await Workflow.ExecuteActivityAsync(
            () => DeviceActivities.Transition(_currentState, target),
            TransitionActivityOptions);
        _currentState = target;
        _transitionCount++;
    }

    [WorkflowSignal]
    public async Task Stop()
    {
        await Workflow.ExecuteActivityAsync(
            () => DeviceActivities.Stop(_currentState),
            TransitionActivityOptions);
        throw new ApplicationFailureException("Device stopped");
    }
}
