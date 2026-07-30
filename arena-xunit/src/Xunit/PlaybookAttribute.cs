using System;
using System.Runtime.CompilerServices;

namespace ArenaXunit;

[AttributeUsage(AttributeTargets.Method, AllowMultiple = true, Inherited = true)]
public class PlaybookAttribute : Attribute
{
    public Type PlaybookType { get; }

    public PlaybookAttribute(Type playbookType)
    {
        PlaybookType = playbookType ?? throw new ArgumentNullException(nameof(playbookType));
    }
}
