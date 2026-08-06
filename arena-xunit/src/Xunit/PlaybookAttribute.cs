using System;

namespace ArenaDotnet.Xunit;

[AttributeUsage(AttributeTargets.Method | AttributeTargets.Class, AllowMultiple = true, Inherited = true)]
public class PlaybookAttribute : Attribute
{
    public Type PlaybookType { get; }

    public PlaybookAttribute(Type playbookType)
    {
        PlaybookType = playbookType ?? throw new ArgumentNullException(nameof(playbookType));
    }
}
