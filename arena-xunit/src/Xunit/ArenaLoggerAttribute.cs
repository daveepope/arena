using System;
using ArenaDotnet.Xunit.Ffi;

namespace ArenaDotnet.Xunit;

[AttributeUsage(AttributeTargets.Field, Inherited = false)]
public sealed class ArenaLoggerAttribute : Attribute
{
    public ArenaLogLevel Level { get; set; } = ArenaLogLevel.Info;
}
