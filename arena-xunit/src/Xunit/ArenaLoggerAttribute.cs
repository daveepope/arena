using System;
using ArenaXunit.Ffi;
using Microsoft.Extensions.Logging;

namespace ArenaXunit;

[AttributeUsage(AttributeTargets.Field)]
public class ArenaLoggerAttribute : Attribute
{
    public ArenaLogLevel Level { get; set; } = ArenaLogLevel.Info;
}
