namespace ArenaExamples.Readings.Aspnet.Models;

public class DeviceStateResponse
{
    public int DeviceId { get; set; }
    public string State { get; set; } = default!;
    public int TransitionCount { get; set; }
}
