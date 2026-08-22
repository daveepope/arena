using System;

namespace ArenaExamples.Readings.Aspnet.Models;

public class WeatherReportRow
{
    public long Id { get; set; }
    public DateTime RecordedAt { get; set; }
    public double Precipitation { get; set; }
    public double Humidity { get; set; }
    public double Pressure { get; set; }
}
