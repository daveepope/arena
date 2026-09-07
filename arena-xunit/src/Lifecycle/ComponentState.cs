using System.Collections.Generic;

namespace ArenaDotnet.Xunit.Lifecycle;

public sealed class ComponentState
{
    public string Id { get; set; } = "";
    public string State { get; set; } = "";
    public List<Fault> Faults { get; set; } = new();
    public List<ComponentState> Children { get; set; } = new();

    public ComponentState? Find(string identifier)
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
