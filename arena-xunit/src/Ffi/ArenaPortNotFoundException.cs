namespace ArenaDotnet.Xunit.Ffi;

public sealed class ArenaPortNotFoundException : ArenaBindingError
{
    public ArenaPortNotFoundException(string message) : base(message) { }
}
