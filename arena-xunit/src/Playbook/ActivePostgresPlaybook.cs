using System;
using ArenaDotnet.Xunit.Ffi;
using ArenaDotnet.Xunit.Support;
using Newtonsoft.Json;

namespace ArenaDotnet.Xunit.Playbook;

public sealed class ActivePostgresPlaybook : ActivePlaybook
{
    public ActivePostgresPlaybook(IntPtr handle) : base(handle) { }

    public void Verify(string query, int expectedValue)
    {
        var spec = ArenaJson.Serialize(new PostgresVerifySpec
        {
            Query = query,
            ExpectedValue = expectedValue,
        });
        ArenaBindings.PostgresPlaybookVerify(_handle, spec);
    }

    [JsonObject(ItemNullValueHandling = NullValueHandling.Ignore)]
    private sealed class PostgresVerifySpec
    {
        [JsonProperty("query")] public string? Query { get; set; }
        [JsonProperty("expected_value")] public int ExpectedValue { get; set; }
    }
}
