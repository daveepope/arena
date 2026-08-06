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
            await Task.Delay(1000);
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

    [WorkflowSignal]
    public async Task RequestTransition(string target)
    {
        if (_currentState == target)
            return;

        await DeviceActivities.Transition(_currentState, target);
        _currentState = target;
        _transitionCount++;
    }

    [WorkflowSignal]
    public async Task Stop()
    {
        await DeviceActivities.Stop(_currentState);
        throw new ApplicationFailureException("Device stopped");
    }
}
