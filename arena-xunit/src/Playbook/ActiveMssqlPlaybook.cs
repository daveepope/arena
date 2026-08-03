using System;
using ArenaXunit.Ffi;
using ArenaXunit.Support;
using Newtonsoft.Json;

namespace ArenaXunit.Playbook;

public sealed class ActiveMssqlPlaybook : ActivePlaybook
{
    public ActiveMssqlPlaybook(IntPtr handle) : base(handle) { }

    public void Verify(string query, int expectedValue)
    {
        var spec = ArenaJson.Serialize(new MssqlVerifySpec
        {
            Query = query,
            ExpectedValue = expectedValue,
        });
        ArenaBindings.MssqlPlaybookVerify(_handle, spec);
    }

    [JsonObject(ItemNullValueHandling = NullValueHandling.Ignore)]
    private sealed class MssqlVerifySpec
    {
        [JsonProperty("query")] public string? Query { get; set; }
        [JsonProperty("expected_value")] public int ExpectedValue { get; set; }
    }
}
