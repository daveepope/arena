using System.Diagnostics;

namespace ArenaExamples.Readings.Aspnet.Workflows;

public static class DeviceActivities
{
    [Temporal.Activity]
    public static async Task Transition(string fromState, string toState)
    {
        Debug.WriteLine($"Device transitioning {fromState} -> {toState}");
        await Task.Delay(10);
    }

    [Temporal.Activity]
    public static async Task Stop(string currentState)
    {
        Debug.WriteLine($"Device stopping from state {currentState}");
        await Task.Delay(10);
    }
}
