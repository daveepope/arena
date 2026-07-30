namespace ArenaXunit.Ffi;

public sealed class ArenaBindingError : System.Exception
{
    public ArenaBindingError(string message) : base(message) { }
    public ArenaBindingError(string message, System.Exception? inner) : base(message, inner) { }
}
