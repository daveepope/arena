using System;

namespace ArenaDotnet.Xunit;

[AttributeUsage(AttributeTargets.Field, Inherited = false)]
public sealed class ArenaComponentAttribute : Attribute
{
    public bool Logs { get; set; } = false;
}
