using System;
using ArenaXunit.Ffi;
using ArenaXunit.Support;
using Newtonsoft.Json;

namespace ArenaXunit.Playbook;

public sealed class ActiveMssqlPlaybook : ActivePlaybook
{
    public ActiveMssqlPlaybook(IntPtr handle) : base(handle) { }

    public void Verify(string query, string expectedValue)
    {
        var spec = ArenaJson.Serialize(new MssqlVerifySpec
        {
            Query = query,
            ExpectedValue = expectedValue,
        });
        ArenaBindings.MssqlPlaybookVerify(GetHandle(), spec);
    }

    private IntPtr GetHandle()
    {
        var field = this.GetType().GetField("_handle",
            System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Instance);
        return field != null ? (IntPtr)field.GetValue(this)! : IntPtr.Zero;
    }

    [JsonObject(ItemNullValueHandling = NullValueHandling.Ignore)]
    private sealed class MssqlVerifySpec
    {
        [JsonProperty("query")] public string? Query { get; set; }
        [JsonProperty("expected_value")] public string? ExpectedValue { get; set; }
    }
}
