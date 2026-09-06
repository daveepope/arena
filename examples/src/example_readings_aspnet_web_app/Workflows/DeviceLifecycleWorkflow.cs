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
            await Workflow.DelayAsync(TimeSpan.FromMilliseconds(100));
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
        StartToCloseTimeout = TimeSpan.FromMilliseconds(100),
    };

    [WorkflowUpdate]
    public async Task<string> RequestTransition(string target)
    {
        if (_currentState == target)
            return QuerySnapshot();

        await Workflow.ExecuteActivityAsync(
            () => DeviceActivities.Transition(_currentState, target),
            TransitionActivityOptions);
        _currentState = target;
        _transitionCount++;
        return QuerySnapshot();
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
