using System;
using System.Collections.Generic;
using System.Text.RegularExpressions;
using System.Threading.Tasks;
using ArenaExamples.Readings.Aspnet.Models;
using Oracle.ManagedDataAccess.Client;

namespace ArenaExamples.Readings.Aspnet.Services;

public interface IWeatherService
{
    Task<List<WeatherReportRow>> GetAllAsync();
    Task<CreateWeatherReportResponse> CreateAsync(CreateWeatherReportRequest request);
}

public class WeatherService : IWeatherService
{
    private static readonly Regex EasyConnect = new(@"^([^/]+)/(.+)@([^@]+)$");

    private readonly string _oracleConnectionString;

    public WeatherService(string oracleConnectionString)
    {
        _oracleConnectionString = BuildOracleConnectionString(oracleConnectionString);
    }

    public async Task<List<WeatherReportRow>> GetAllAsync()
    {
        var results = new List<WeatherReportRow>();

        using var conn = new OracleConnection(_oracleConnectionString);
        await conn.OpenAsync();

        using var cmd = conn.CreateCommand();
        cmd.CommandText = @"
            SELECT id, recorded_at, precipitation, humidity, pressure
            FROM weather_report
            ORDER BY id DESC
            FETCH FIRST 50 ROWS ONLY";

        using var reader = await cmd.ExecuteReaderAsync();
        while (await reader.ReadAsync())
        {
            results.Add(new WeatherReportRow
            {
                Id = reader.GetInt64(0),
                RecordedAt = reader.GetDateTime(1),
                Precipitation = reader.GetDouble(2),
                Humidity = reader.GetDouble(3),
                Pressure = reader.GetDouble(4)
            });
        }

        return results;
    }

    public async Task<CreateWeatherReportResponse> CreateAsync(CreateWeatherReportRequest request)
    {
        using var conn = new OracleConnection(_oracleConnectionString);
        await conn.OpenAsync();

        using var cmd = conn.CreateCommand();
        cmd.CommandText = @"
            INSERT INTO weather_report (recorded_at, precipitation, humidity, pressure)
            VALUES (:recordedAt, :precipitation, :humidity, :pressure)
            RETURNING id INTO :id";
        cmd.BindByName = true;
        cmd.Parameters.Add(new OracleParameter("recordedAt", OracleDbType.TimeStamp) { Value = DateTime.UtcNow });
        cmd.Parameters.Add(new OracleParameter("precipitation", OracleDbType.Double) { Value = request.Precipitation });
        cmd.Parameters.Add(new OracleParameter("humidity", OracleDbType.Double) { Value = request.Humidity });
        cmd.Parameters.Add(new OracleParameter("pressure", OracleDbType.Double) { Value = request.Pressure });
        var idParam = new OracleParameter("id", OracleDbType.Int64, System.Data.ParameterDirection.Output);
        cmd.Parameters.Add(idParam);

        await cmd.ExecuteNonQueryAsync();

        return new CreateWeatherReportResponse { Id = Convert.ToInt64(idParam.Value.ToString()) };
    }

    private static string BuildOracleConnectionString(string easyConnect)
    {
        var match = EasyConnect.Match(easyConnect);
        if (!match.Success)
            throw new ArgumentException("oracle connection string incomplete");
        var user = match.Groups[1].Value;
        var password = match.Groups[2].Value;
        var dsn = match.Groups[3].Value;
        return $"User Id={user};Password={password};Data Source={dsn};";
    }
}
