using System;

namespace ArenaXunit;

[AttributeUsage(AttributeTargets.Field | AttributeTargets.Method | AttributeTargets.Class, AllowMultiple = true, Inherited = true)]
public class PlaybookAttribute : Attribute
{
    public Type PlaybookType { get; }
    public bool ExecOnDependencyStart { get; set; } = true;

    public PlaybookAttribute(Type playbookType)
    {
        PlaybookType = playbookType ?? throw new ArgumentNullException(nameof(playbookType));
    }
}
