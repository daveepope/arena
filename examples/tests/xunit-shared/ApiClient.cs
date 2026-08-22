using System;
using System.Collections.Generic;
using System.Net;
using System.Net.Http;
using System.Net.Http.Json;
using System.Text.Json;
using System.Threading.Tasks;
using Polly;
using Polly.Retry;

namespace ArenaExamples.Test.Shared;

public class ApiClient
{
    private static readonly JsonSerializerOptions SerializerOptions = new() { PropertyNameCaseInsensitive = true };

    private static readonly ResiliencePipeline<HttpResponseMessage> GetPipeline =
        new ResiliencePipelineBuilder<HttpResponseMessage>()
            .AddTimeout(TimeSpan.FromSeconds(3))
            .AddRetry(new RetryStrategyOptions<HttpResponseMessage>
            {
                ShouldHandle = new PredicateBuilder<HttpResponseMessage>()
                    .HandleResult(response => !response.IsSuccessStatusCode)
                    .Handle<HttpRequestException>(),
                MaxRetryAttempts = int.MaxValue,
                BackoffType = DelayBackoffType.Constant,
                Delay = TimeSpan.FromMilliseconds(100),
            })
            .Build();

    private readonly HttpClient _client;

    public ApiClient(string baseUrl, string accessToken)
    {
        _client = new HttpClient
        {
            BaseAddress = new Uri(baseUrl),
        };
        _client.DefaultRequestHeaders.Authorization =
            new System.Net.Http.Headers.AuthenticationHeaderValue("Bearer", accessToken);
    }

    public async Task<string> GetAsync(string path)
    {
        var response = await GetRawAsync(path);
        response.EnsureSuccessStatusCode();
        return await response.Content.ReadAsStringAsync();
    }

    public async Task<TResponse> GetJsonAsync<TResponse>(string path)
    {
        var json = await GetAsync(path);
        return JsonSerializer.Deserialize<TResponse>(json, SerializerOptions)!;
    }

    public async Task<TResponse> PostJsonAsync<TRequest, TResponse>(string path, TRequest request)
    {
        var response = await _client.PostAsJsonAsync(path, request);
        response.EnsureSuccessStatusCode();
        var json = await response.Content.ReadAsStringAsync();
        return JsonSerializer.Deserialize<TResponse>(json, SerializerOptions)!;
    }

    public async Task<HttpResponseMessage> PostJsonRawAsync<TRequest>(string path, TRequest request)
    {
        return await _client.PostAsJsonAsync(path, request);
    }

    public async Task DeleteAsync(string path)
    {
        await _client.DeleteAsync(path);
    }

    public async Task<HttpResponseMessage> GetRawAsync(string path)
    {
        return await GetPipeline.ExecuteAsync(async _ => await _client.GetAsync(path));
    }

    public async Task<bool> StopDeviceAsync(int deviceId)
    {
        var response = await _client.DeleteAsync($"/Devices/{deviceId}");
        return response.IsSuccessStatusCode || response.StatusCode == System.Net.HttpStatusCode.NoContent;
    }

    public async Task<CreateReadingResponse> CreateReadingAsync(CreateReadingRequest request)
    {
        return await PostJsonAsync<CreateReadingRequest, CreateReadingResponse>("/Readings", request);
    }

    public async Task<HttpResponseMessage> PostReadingRawAsync(CreateReadingRequest request)
    {
        return await PostJsonRawAsync<CreateReadingRequest>("/Readings", request);
    }

    public async Task<CreateDeviceResponse> CreateDeviceAsync(CreateDeviceRequest request)
    {
        return await PostJsonAsync<CreateDeviceRequest, CreateDeviceResponse>("/Devices", request);
    }

    public async Task SetDeviceStateAsync(int deviceId, SetDeviceStateRequest request)
    {
        var response = await _client.PostAsJsonAsync($"/Devices/{deviceId}/state", request);
        response.EnsureSuccessStatusCode();
    }

    public async Task<DeviceStateResponse> GetDeviceStateAsync(int deviceId)
    {
        return await GetJsonAsync<DeviceStateResponse>($"/Devices/{deviceId}/state");
    }

    public async Task<HttpResponseMessage> GetDeviceStateRawAsync(object deviceId)
    {
        return await GetRawAsync($"/Devices/{deviceId}/state");
    }

    public async Task<List<ReadingRow>> ListReadingsAsync()
    {
        return await GetJsonAsync<List<ReadingRow>>("/Readings");
    }

    public async Task<CreateWeatherReportResponse> CreateWeatherReportAsync(CreateWeatherReportRequest request)
    {
        return await PostJsonAsync<CreateWeatherReportRequest, CreateWeatherReportResponse>("/Weather", request);
    }

    public async Task<List<WeatherReportRow>> ListWeatherReportsAsync()
    {
        return await GetJsonAsync<List<WeatherReportRow>>("/Weather");
    }
}

public class CreateReadingRequest
{
    public string UserName { get; set; } = default!;
    public int Value { get; set; }
    public string? Comment { get; set; }
    public int DeviceId { get; set; }
}

public class CreateReadingResponse
{
    public int Id { get; set; }
    public bool Valid { get; set; }
}

public class CreateDeviceRequest
{
    public string Name { get; set; } = default!;
}

public class CreateDeviceResponse
{
    public int Id { get; set; }
    public string Name { get; set; } = default!;
}

public class SetDeviceStateRequest
{
    public string Target { get; set; } = default!;
}

public class DeviceStateResponse
{
    public int DeviceId { get; set; }
    public string State { get; set; } = default!;
    public int TransitionCount { get; set; }
}

public class ReadingRow
{
    public int Id { get; set; }
    public string UserName { get; set; } = default!;
    public int Value { get; set; }
    public string? Comment { get; set; }
}

public class CreateWeatherReportRequest
{
    public double Precipitation { get; set; }
    public double Humidity { get; set; }
    public double Pressure { get; set; }
}

public class CreateWeatherReportResponse
{
    public long Id { get; set; }
}

public class WeatherReportRow
{
    public long Id { get; set; }
    public DateTime RecordedAt { get; set; }
    public double Precipitation { get; set; }
    public double Humidity { get; set; }
    public double Pressure { get; set; }
}
