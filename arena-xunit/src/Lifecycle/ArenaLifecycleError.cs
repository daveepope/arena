using ArenaDotnet.Xunit.Ffi;

namespace ArenaDotnet.Xunit.Lifecycle;

public class ArenaLifecycleError : ArenaBindingError
{
    public ArenaState? State { get; }

    public ArenaLifecycleError(string message, ArenaState? state) : base(message)
    {
        State = state;
    }

    public static ArenaBindingError From(ArenaBindingError error)
    {
        if (error is ArenaLifecycleError)
            return error;
        var document = error.StateDocument;
        if (string.IsNullOrEmpty(document))
            return error;
        ArenaState state;
        try
        {
            state = ArenaState.Parse(document!);
        }
        catch (System.ArgumentException)
        {
            return error;
        }
        return new ArenaLifecycleError(error.Message, state);
    }
}
