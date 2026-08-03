using System;

namespace ArenaXunit;

[AttributeUsage(AttributeTargets.Field, Inherited = false)]
public sealed class ArenaPlaybookAttribute : Attribute
{
    public bool ExecOnDependencyStart { get; set; } = true;
}
