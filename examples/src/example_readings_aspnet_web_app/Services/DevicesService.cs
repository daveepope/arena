using ArenaExamples.Readings.Aspnet.Models;

namespace ArenaExamples.Readings.Aspnet.Services;

public interface IDevicesService
{
    Task<List<DeviceRow>> GetAllAsync();
    Task<CreateDeviceResponse> CreateAsync(CreateDeviceRequest request);
    Task<DeviceStateResponse?> GetStateAsync(int deviceId);
    Task<DeviceStateResponse?> SetStateAsync(int deviceId, string target);
    Task<bool> DeleteAsync(int deviceId);
}

public class DevicesService : IDevicesService
{
    private readonly string _postgresConnectionString;
    private readonly IDeviceWorkflowService _workflowService;
    private readonly ISmtpClientService _smtpService;

    public DevicesService(
        string postgresConnectionString,
        IDeviceWorkflowService workflowService,
        ISmtpClientService smtpService)
    {
        _postgresConnectionString = postgresConnectionString;
        _workflowService = workflowService;
        _smtpService = smtpService;
    }

    public async Task<List<DeviceRow>> GetAllAsync()
    {
        var results = new List<DeviceRow>();
        using var conn = new Npgsql.NpgsqlConnection(_postgresConnectionString);
        await conn.OpenAsync();
        using var cmd = conn.CreateCommand();
        cmd.CommandText = @"SELECT id, name FROM instrument_reading.device ORDER BY id";
        using var reader = await cmd.ExecuteReaderAsync();
        while (await reader.ReadAsync())
        {
            results.Add(new DeviceRow
            {
                Id = reader.GetInt32(0),
                Name = reader.GetString(1)
            });
        }
        return results;
    }

    public async Task<CreateDeviceResponse> CreateAsync(CreateDeviceRequest request)
    {
        var conn = new Npgsql.NpgsqlConnection(_postgresConnectionString);
        await conn.OpenAsync();
        using var tx = conn.BeginTransaction();
        int deviceId;
        try
        {
            using (var cmd = conn.CreateCommand())
            {
                cmd.Transaction = tx;
                cmd.CommandText = @"
                    INSERT INTO instrument_reading.device (name)
                    VALUES (@name)
                    RETURNING id";
                cmd.Parameters.AddWithValue("@name", request.Name);
                deviceId = (int)(await cmd.ExecuteScalarAsync()!)!;
            }

            await _workflowService.StartDeviceAsync(deviceId);
            await tx.CommitAsync();
        }
        catch
        {
            await tx.RollbackAsync();
            throw;
        }

        await _smtpService.SendAsync(
            $"device+{deviceId}@arena.local",
            $"Device {deviceId} Provisioned",
            $"Device '{request.Name}' has been provisioned successfully.");

        return new CreateDeviceResponse
        {
            Id = deviceId,
            Name = request.Name
        };
    }

    public Task<DeviceStateResponse?> GetStateAsync(int deviceId)
    {
        return _workflowService.GetStateAsync(deviceId);
    }

    public async Task<DeviceStateResponse?> SetStateAsync(int deviceId, string target)
    {
        var valid = target == "ON" || target == "OFF" || target == "ERROR";
        if (!valid)
            return null;

        var signaled = await _workflowService.SignalTransitionAsync(deviceId, target);
        if (!signaled)
            return null;

        var deadline = DateTime.UtcNow.AddMilliseconds(500);
        while (DateTime.UtcNow < deadline)
        {
            var state = await _workflowService.GetStateAsync(deviceId);
            if (state != null && state.State == target)
                return state;
            await Task.Delay(50);
        }

        return await _workflowService.GetStateAsync(deviceId);
    }

    public Task<bool> DeleteAsync(int deviceId)
    {
        return _workflowService.StopDeviceAsync(deviceId);
    }
}
