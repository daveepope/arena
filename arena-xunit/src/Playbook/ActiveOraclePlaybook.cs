using System;
using ArenaDotnet.Xunit.Ffi;
using ArenaDotnet.Xunit.Support;
using Newtonsoft.Json;

namespace ArenaDotnet.Xunit.Playbook;

public sealed class ActiveOraclePlaybook : ActivePlaybook
{
    public ActiveOraclePlaybook(IntPtr handle) : base(handle) { }

    public void Verify(string query, int expectedValue)
    {
        var spec = ArenaJson.Serialize(new OracleVerifySpec
        {
            Query = query,
            ExpectedValue = expectedValue,
        });
        ArenaBindings.OraclePlaybookVerify(_handle, spec);
    }

    [JsonObject(ItemNullValueHandling = NullValueHandling.Ignore)]
    private sealed class OracleVerifySpec
    {
        [JsonProperty("query")] public string? Query { get; set; }
        [JsonProperty("expected_value")] public int ExpectedValue { get; set; }
    }
}
