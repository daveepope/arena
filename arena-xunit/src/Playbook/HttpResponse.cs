using System.Collections.Generic;
using Newtonsoft.Json;

namespace ArenaDotnet.Xunit.Playbook;

public static class HttpResponse
{
    public static HttpResponseObj Ok() => Status(200, null);
    public static HttpResponseObj OkJson(object json) => StatusJson(200, json);
    public static HttpResponseObj Created() => Status(201, null);
    public static HttpResponseObj NoContent() => Status(204, null);
    public static HttpResponseObj Status(int code) => Status(code, null);
    public static HttpResponseObj Status(int code, string? body) => new(code, body);
    public static HttpResponseObj StatusJson(int code, object json) => new(code, json);
    public static HttpResponseObj ServerError() => Status(500, null);
}

[JsonObject(ItemNullValueHandling = NullValueHandling.Ignore)]
public sealed class HttpResponseObj
{
    [JsonProperty("status")] public int Status { get; }
    [JsonProperty("json_body")] public object? JsonBody { get; }

    public HttpResponseObj(int status, object? jsonBody)
    {
        Status = status;
        JsonBody = jsonBody;
    }
}
