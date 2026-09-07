using System.Collections.Generic;

namespace ArenaDotnet.Xunit.Lifecycle;

public sealed class Fault
{
    public string Id { get; set; } = "";
    public string Subject { get; set; } = "";
    public string Message { get; set; } = "";
    public string At { get; set; } = "";
    public List<Fault> Faults { get; set; } = new();
}
