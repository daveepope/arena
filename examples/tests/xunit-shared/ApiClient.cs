using System;
using System.Collections.Generic;
using System.Net;
using System.Net.Http;
using System.Net.Http.Json;
using System.Text.Json;
using System.Threading.Tasks;

namespace ArenaExamples.Test.Shared;

public class ApiClient
{
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
        var response = await _client.GetAsync(path);
        response.EnsureSuccessStatusCode();
        return await response.Content.ReadAsStringAsync();
    }

    public async Task<TResponse> GetJsonAsync<TResponse>(string path)
    {
        var json = await GetAsync(path);
        return JsonSerializer.Deserialize<TResponse>(json)!;
    }

    public async Task<TResponse> PostJsonAsync<TRequest, TResponse>(string path, TRequest request)
    {
        var response = await _client.PostAsJsonAsync(path, request);
        response.EnsureSuccessStatusCode();
        var json = await response.Content.ReadAsStringAsync();
        return JsonSerializer.Deserialize<TResponse>(json)!;
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
        return await _client.GetAsync(path);
    }

    public async Task<bool> StopDeviceAsync(int deviceId)
    {
        var response = await _client.DeleteAsync($"/devices/{deviceId}");
        return response.IsSuccessStatusCode || response.StatusCode == System.Net.HttpStatusCode.NoContent;
    }

    public async Task<CreateReadingResponse> CreateReadingAsync(CreateReadingRequest request)
    {
        return await PostJsonAsync<CreateReadingRequest, CreateReadingResponse>("/readings", request);
    }

    public async Task<HttpResponseMessage> PostReadingRawAsync(CreateReadingRequest request)
    {
        return await PostJsonRawAsync<CreateReadingRequest>("/readings", request);
    }

    public async Task<CreateDeviceResponse> CreateDeviceAsync(CreateDeviceRequest request)
    {
        return await PostJsonAsync<CreateDeviceRequest, CreateDeviceResponse>("/devices", request);
    }

    public async Task RequestStateTransitionAsync(int deviceId, DeviceStateTransitionRequest request)
    {
        var response = await _client.PostAsJsonAsync($"/devices/{deviceId}/transition", request);
        response.EnsureSuccessStatusCode();
    }

    public async Task<DeviceStateResponse> GetDeviceAsync(int deviceId)
    {
        return await GetJsonAsync<DeviceStateResponse>($"/devices/{deviceId}");
    }

    public async Task<HttpResponseMessage> GetDeviceRawAsync(object deviceId)
    {
        return await GetRawAsync($"/devices/{deviceId}");
    }

    public async Task<List<CreateReadingResponse>> ListReadingsAsync(string deviceId)
    {
        return await GetJsonAsync<List<CreateReadingResponse>>($"/readings?device_id={deviceId}");
    }
}

public class CreateReadingRequest
{
    public string DeviceId { get; set; } = default!;
    public double TemperatureC { get; set; }
}

public class CreateReadingResponse
{
    public string? Id { get; set; }
    public string DeviceId { get; set; } = default!;
    public double TemperatureC { get; set; }
    public string Status { get; set; } = default!;
    public string? Error { get; set; }
}

public class CreateDeviceRequest
{
    public string Name { get; set; } = default!;
    public string Location { get; set; } = default!;
    public string Type { get; set; } = default!;
    public string TargetState { get; set; } = default!;
}

public class CreateDeviceResponse
{
    public int Id { get; set; }
    public string Name { get; set; } = default!;
    public string Location { get; set; } = default!;
    public string Type { get; set; } = default!;
    public string State { get; set; } = default!;
}

public class DeviceStateTransitionRequest
{
    public string TargetState { get; set; } = default!;
}

public class DeviceStateResponse
{
    public int Id { get; set; }
    public string Name { get; set; } = default!;
    public string State { get; set; } = default!;
    public string? Error { get; set; }
}
