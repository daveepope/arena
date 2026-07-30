using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

namespace ArenaXunit.Support;

internal static class ArenaJson
{
    private static readonly JsonSerializerSettings SerializerSettings = new()
    {
        ContractResolver = new Newtonsoft.Json.Serialization.CamelCasePropertyNamesContractResolver(),
        NullValueHandling = NullValueHandling.Ignore,
    };

    public static JObject Object()
    {
        return new JObject();
    }

    public static string Serialize(object value)
    {
        return JsonConvert.SerializeObject(value, Formatting.None, SerializerSettings);
    }

    public static string Serialize(JToken value)
    {
        return value.ToString(Formatting.None);
    }

    public static T Deserialize<T>(string json)
    {
        return JsonConvert.DeserializeObject<T>(json, SerializerSettings)
            ?? throw new System.InvalidOperationException($"Failed to deserialize {typeof(T).Name}");
    }
}
