using System.Threading.Tasks;
using System.Diagnostics;

namespace ArenaExamples.Readings.Aspnet.Workflows;

public static class DeviceActivities
{
    [Temporalio.Activities.Activity]
    public static Task Transition(string fromState, string toState)
    {
        Debug.WriteLine($"Device transitioning {fromState} -> {toState}");
        return Task.CompletedTask;
    }

    [Temporalio.Activities.Activity]
    public static Task Stop(string currentState)
    {
        Debug.WriteLine($"Device stopping from state {currentState}");
        return Task.CompletedTask;
    }
}
