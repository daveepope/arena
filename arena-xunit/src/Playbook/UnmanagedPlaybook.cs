namespace ArenaDotnet.Xunit.Playbook;

public abstract class UnmanagedPlaybook : IPlaybook
{
    public abstract string Identifier { get; }

    public abstract ActivePlaybook Run(OpenArena arena);
}
