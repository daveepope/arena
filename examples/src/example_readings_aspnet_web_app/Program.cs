using System;
using System.Net.Http;
using ArenaExamples.Readings.Aspnet.Services;
using Temporalio.Client;
using Microsoft.AspNetCore.Builder;
using Microsoft.AspNetCore.Hosting;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;

var builder = WebApplication.CreateBuilder(args);

var port = int.Parse(builder.Configuration["WEB_APP_PORT"] ?? "3010");
builder.WebHost.UseUrls($"http://0.0.0.0:{port}");

builder.Services.AddControllers();
builder.Services.AddHttpClient("JwtValidation", c =>
{
    var tlsCaFile = builder.Configuration["OAUTH_TLS_CA_FILE"];
    var tlsCaPem = builder.Configuration["OAUTH_TLS_CA_PEM"];
    if (!string.IsNullOrEmpty(tlsCaFile))
    {
        var cert = new System.Security.Cryptography.X509Certificates.X509Certificate2(tlsCaFile);
        c.DefaultRequestHeaders.Add("User-Agent", "arena-readings-aspnet");
        c.BaseAddress = new Uri(builder.Configuration["OAUTH_ISSUER_URL"]!);
        if (cert is not null)
        {
            var handler = new System.Net.Http.HttpClientHandler();
            handler.ServerCertificateCustomValidationCallback = (msg, cert2, chain, err) => err == System.Net.Security.SslPolicyErrors.None || cert2 == cert;
            c = new System.Net.Http.HttpClient(handler);
        }
    }
});

builder.Services.AddSingleton<IEventBridgePublisher>(_ => new EventBridgePublisher(
    builder.Configuration["AWS_ENDPOINT_URL"]!,
    builder.Configuration["EVENT_BUS_NAME"]!,
    builder.Configuration["EVENT_SOURCE"]!));
builder.Services.AddSingleton<IReadingsService>(sp => new ReadingsService(
    builder.Configuration["POSTGRES_CONNECTION_STRING"]!,
    builder.Configuration["MSSQL_CONNECTION_STRING"]!,
    builder.Configuration["CALIBRATION_URL"]!,
    sp.GetRequiredService<IEventBridgePublisher>()));
builder.Services.AddSingleton<ISmtpClientService>(_ => new SmtpClientService(
    builder.Configuration["SMTP_HOST"]!,
    int.Parse(builder.Configuration["SMTP_PORT"]!)));

var temporalClient = await TemporalClient.ConnectAsync(new TemporalClientConnectOptions
{
    TargetHost = builder.Configuration["TEMPORAL_TARGET"]
});
builder.Services.AddSingleton(temporalClient);
builder.Services.AddSingleton<IDeviceWorkflowService, DeviceWorkflowService>();
builder.Services.AddSingleton<IDevicesService, DevicesService>();

builder.Services.AddHttpClient();

var app = builder.Build();

app.UseMiddleware<JwtAuthMiddleware>();
app.MapControllers();

app.Run();
