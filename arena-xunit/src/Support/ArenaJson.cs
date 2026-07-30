using Newtonsoft.Json;

namespace ArenaXunit.Support;

internal static class ArenaJson
{
    private static readonly JsonSerializerSettings SerializerSettings = new()
    {
        ContractResolver = new Newtonsoft.Json.Serialization.CamelCasePropertyNamesContractResolver(),
        NullValueHandling = NullValueHandling.Ignore,
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
