namespace ArenaXunit.Playbook;

public interface IPlaybook
{
    string Identifier { get; }
    ActivePlaybook Run(OpenArena arena);
}
