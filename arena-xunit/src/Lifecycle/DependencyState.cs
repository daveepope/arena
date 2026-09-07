using System.Collections.Generic;

namespace ArenaDotnet.Xunit.Lifecycle;

public sealed class DependencyState
{
    public string Id { get; set; } = "";
    public string State { get; set; } = "";
    public List<Fault> Faults { get; set; } = new();
    public List<DependencyState> Children { get; set; } = new();

    public DependencyState? Find(string identifier)
    {
        if (Id == identifier)
            return this;
        foreach (var child in Children)
        {
            var found = child.Find(identifier);
            if (found != null)
                return found;
        }
        return null;
    }
}
