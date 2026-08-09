namespace ArenaDotnet.Xunit;

public interface IArenaDependency
{
    string Identifier { get; }
    string ForFfi();
}
