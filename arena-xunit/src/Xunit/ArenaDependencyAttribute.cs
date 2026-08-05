using System;

namespace ArenaDotnet.Xunit;

[AttributeUsage(AttributeTargets.Field, Inherited = false)]
public sealed class ArenaDependencyAttribute : Attribute
{
    public bool Logs { get; set; } = false;
}
