using System;
using ArenaXunit.Ffi;

namespace ArenaXunit;

[AttributeUsage(AttributeTargets.Field, Inherited = false)]
public sealed class ArenaLoggerAttribute : Attribute
{
    public ArenaLogLevel Level { get; set; } = ArenaLogLevel.Info;
}
