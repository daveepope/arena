using Newtonsoft.Json;

namespace ArenaDotnet.Xunit;

public interface IArenaMatchPiece
{
    string Identifier { get; }
    string ForFfi();
}
