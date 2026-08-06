using Newtonsoft.Json;
using Newtonsoft.Json.Converters;
using Newtonsoft.Json.Serialization;

namespace ArenaDotnet.Xunit.Support;

internal static class ArenaJson
{
    private static readonly JsonSerializerSettings SerializerSettings = new()
    {
        ContractResolver = new DefaultContractResolver
        {
            NamingStrategy = new SnakeCaseNamingStrategy(processDictionaryKeys: false, overrideSpecifiedNames: true),
        },
        NullValueHandling = NullValueHandling.Ignore,
        Converters = { new StringEnumConverter(new SnakeCaseNamingStrategy()) },
    };

    public static string Serialize(object value)
    {
        return JsonConvert.SerializeObject(value, Formatting.None, SerializerSettings);
    }

    public static T Deserialize<T>(string json)
    {
        return JsonConvert.DeserializeObject<T>(json, SerializerSettings)
            ?? throw new System.InvalidOperationException($"Failed to deserialize {typeof(T).Name}");
    }
}
