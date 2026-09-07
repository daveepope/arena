namespace ArenaDotnet.Xunit.Ffi;

public class ArenaBindingError : System.Exception
{
    public string? StateDocument { get; }

    public ArenaBindingError(string message) : base(message) { }
    public ArenaBindingError(string message, System.Exception? inner) : base(message, inner) { }

    public ArenaBindingError(string message, string? stateDocument) : base(message)
    {
        StateDocument = stateDocument;
    }
}
