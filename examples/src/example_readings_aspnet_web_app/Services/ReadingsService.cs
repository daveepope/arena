using System.Data.Common;
using System.Net.Http.Json;
using System.Text.Json;
using ArenaExamples.Readings.Aspnet.Models;

namespace ArenaExamples.Readings.Aspnet.Services;

public interface IReadingsService
{
    Task<List<ReadingRow>> GetAllAsync();
    Task<CreateReadingResponse> CreateAsync(CreateReadingRequest request);
}

public class ReadingsService : IReadingsService
{
    private readonly string _postgresConnectionString;
    private readonly string _mssqlConnectionString;
    private readonly string _calibrationBaseUrl;
    private readonly IEventBridgePublisher _eventPublisher;

    public ReadingsService(
        string postgresConnectionString,
        string mssqlConnectionString,
        string calibrationBaseUrl,
        IEventBridgePublisher eventPublisher)
    {
        _postgresConnectionString = postgresConnectionString;
        _mssqlConnectionString = mssqlConnectionString;
        _calibrationBaseUrl = calibrationBaseUrl;
        _eventPublisher = eventPublisher;
    }

    public async Task<List<ReadingRow>> GetAllAsync()
    {
        var results = new List<ReadingRow>();

        using var conn = new Npgsql.NpgsqlConnection(_postgresConnectionString);
        await conn.OpenAsync();

        using var cmd = conn.CreateCommand();
        cmd.CommandText = @"
            SELECT r.id, u.user_name, r.value, r.comment
            FROM instrument_reading.reading r
            JOIN instrument_reading.""user"" u ON r.user_id = u.id
            ORDER BY r.id DESC LIMIT 50";

        using var reader = await cmd.ExecuteReaderAsync();
        while (await reader.ReadAsync())
        {
            results.Add(new ReadingRow
            {
                Id = reader.GetInt32(0),
                UserName = reader.GetString(1),
                Value = reader.GetInt32(2),
                Comment = reader.IsDBNull(3) ? null : reader.GetString(3)
            });
        }

        return results;
    }

    public async Task<CreateReadingResponse> CreateAsync(CreateReadingRequest request)
    {
        bool valid = await ValidateReadingAsync(request.Value);

        await InsertValidationResultAsync(request.UserName, request.Value, valid);

        int userId = await UpsertUserAsync(request.UserName);

        int readingId = await InsertReadingAsync(userId, request.Value, request.Comment, request.DeviceId);

        await PublishReadingCreatedEventAsync(request.UserName, readingId, request.Value, request.Comment, request.DeviceId);

        return new CreateReadingResponse { Valid = valid, Id = readingId };
    }

    private async Task<bool> ValidateReadingAsync(int value)
    {
        try
        {
            using var client = new HttpClient();
            var payload = new { value };
            var response = await client.PostAsJsonAsync($"{_calibrationBaseUrl}/api/v1/validate", payload);
            response.EnsureSuccessStatusCode();
            var content = await response.Content.ReadFromJsonAsync<CalibrationResponse>();
            return content?.Valid ?? false;
        }
        catch
        {
            return false;
        }
    }

    private async Task InsertValidationResultAsync(string userName, int value, bool valid)
    {
        using var conn = new Microsoft.Data.SqlClient.SqlConnection(_mssqlConnectionString);
        await conn.OpenAsync();
        using var cmd = conn.CreateCommand();
        cmd.CommandText = @"INSERT INTO dbo.validation_results (user_name, value, valid, validated_at) VALUES (@userName, @value, @valid, SYSDATETIME())";
        cmd.Parameters.AddWithValue("@userName", userName);
        cmd.Parameters.AddWithValue("@value", value);
        cmd.Parameters.AddWithValue("@valid", valid);
        await cmd.ExecuteNonQueryAsync();
    }

    private async Task<int> UpsertUserAsync(string userName)
    {
        using var conn = new Npgsql.NpgsqlConnection(_postgresConnectionString);
        await conn.OpenAsync();
        using var cmd = conn.CreateCommand();
        cmd.CommandText = @"
            INSERT INTO instrument_reading.""user"" (user_name)
            VALUES (@userName)
            ON CONFLICT (user_name) DO NOTHING
            RETURNING id";

        cmd.Parameters.AddWithValue("@userName", userName);
        var result = await cmd.ExecuteScalarAsync();
        if (result != null && result is int id && id > 0)
            return id;

        using (cmd)
        {
            cmd.CommandText = @"SELECT id FROM instrument_reading.""user"" WHERE user_name = @userName";
            return (int)(await cmd.ExecuteScalarAsync()!)!;
        }
    }

    private async Task<int> InsertReadingAsync(int userId, int value, string? comment, int deviceId)
    {
        using var conn = new Npgsql.NpgsqlConnection(_postgresConnectionString);
        await conn.OpenAsync();
        using var cmd = conn.CreateCommand();
        cmd.CommandText = @"
            INSERT INTO instrument_reading.reading (user_id, value, comment, device_id)
            VALUES (@userId, @value, @comment, @deviceId)
            RETURNING id";

        cmd.Parameters.AddWithValue("@userId", userId);
        cmd.Parameters.AddWithValue("@value", value);
        cmd.Parameters.AddWithValue("@comment", (object?)comment ?? DBNull.Value);
        cmd.Parameters.AddWithValue("@deviceId", deviceId);
        return (int)(await cmd.ExecuteScalarAsync()!)!;
    }

    private Task PublishReadingCreatedEventAsync(string userName, int readingId, int value, string? comment, int deviceId)
    {
        return _eventPublisher.PublishAsync("ReadingCreated", new
        {
            user_name = userName,
            reading_id = readingId,
            value,
            comment,
            device_id = deviceId
        });
    }

    private class CalibrationResponse
    {
        public bool Valid { get; set; }
    }
}
