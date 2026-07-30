namespace ArenaExamples.Readings.Aspnet.Models;

public class ReadingRow
{
    public int Id { get; set; }
    public string UserName { get; set; } = default!;
    public int Value { get; set; }
    public string? Comment { get; set; }
}
