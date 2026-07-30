namespace ArenaExamples.Readings.Aspnet.Models;

public class CreateReadingRequest
{
    public string UserName { get; set; } = default!;
    public int Value { get; set; }
    public string? Comment { get; set; }
    public int DeviceId { get; set; }
}
