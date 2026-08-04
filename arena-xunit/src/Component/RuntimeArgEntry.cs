using System.Collections.Generic;

namespace ArenaDotnet.Xunit.Component;

internal sealed class RuntimeArgEntry
{
    public RuntimeArgEntry(string name, string value)
    {
        Name = name;
        Value = value;
    }

    public string Name { get; }
    public string Value { get; }

    public static List<object> Build(IReadOnlyList<RuntimeArgEntry> entries)
    {
        var result = new List<object>(entries.Count);
        foreach (var entry in entries)
            result.Add(new { name = entry.Name, value = entry.Value });
        return result;
    }
}
