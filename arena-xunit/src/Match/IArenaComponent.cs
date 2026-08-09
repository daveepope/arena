namespace ArenaDotnet.Xunit;

public interface IArenaComponent
{
    string Identifier { get; }
    string ForFfi();
}
