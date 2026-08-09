using System.Collections.Generic;
using System.Linq;
using Newtonsoft.Json.Linq;

namespace ArenaDotnet.Xunit.Support;

internal static class ChildrenWireFormat
{
    public static List<JToken>? Build(IReadOnlyList<IArenaMatchPiece> children)
    {
        return children.Count > 0 ? children.Select(c => JToken.Parse(c.ForFfi())).ToList() : null;
    }
}
