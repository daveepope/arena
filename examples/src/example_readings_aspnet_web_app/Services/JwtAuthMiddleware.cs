using System;
using System.Collections.Generic;
using System.Linq;
using System.Net.Http;
using System.Threading.Tasks;
using System.IdentityModel.Tokens.Jwt;
using System.Net.Http.Headers;
using System.Security.Cryptography.X509Certificates;
using System.Text;
using System.Text.Json;
using Microsoft.AspNetCore.Http;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.Logging;

namespace ArenaExamples.Readings.Aspnet.Services;

public class JwtAuthMiddleware
{
    private readonly RequestDelegate _next;
    private readonly string _issuerUrl;
    private readonly string? _requiredScopes;
    private readonly HttpClient _httpClient;
    private readonly ILogger<JwtAuthMiddleware> _logger;
    private volatile JwkSet? _cachedJwks;

    public JwtAuthMiddleware(RequestDelegate next, IConfiguration config, IHttpClientFactory factory, ILogger<JwtAuthMiddleware> logger)
    {
        _next = next;
        _issuerUrl = config["OAUTH_ISSUER_URL"] ?? throw new InvalidOperationException("OAUTH_ISSUER_URL not set");
        _requiredScopes = config["OAUTH_REQUIRED_ACCESS_TOKEN_SCOPES"];
        _httpClient = factory.CreateClient("JwtValidation");
        _logger = logger;
    }

    public async Task InvokeAsync(HttpContext context)
    {
        if (context.Request.Path.StartsWithSegments("/health", StringComparison.OrdinalIgnoreCase))
        {
            await _next(context);
            return;
        }

        var authHeader = context.Request.Headers["Authorization"].ToString();
        if (string.IsNullOrEmpty(authHeader) || !authHeader.StartsWith("Bearer ", StringComparison.OrdinalIgnoreCase))
        {
            context.Response.StatusCode = 401;
            context.Response.ContentType = "application/json";
            await context.Response.WriteAsync("Unauthorized");
            return;
        }

        var token = authHeader.Substring(7).Trim();

        try
        {
            var tokenHandler = new JwtSecurityTokenHandler();
            var jwks = await GetJwksAsync();
            var validationParameters = new Microsoft.IdentityModel.Tokens.TokenValidationParameters
            {
                RequireExpirationTime = true,
                ValidateLifetime = true,
                ValidIssuer = _issuerUrl,
                ValidateIssuer = true,
                ValidateAudience = false,
                ValidateIssuerSigningKey = true,
                IssuerSigningKeys = jwks.Keys.Select(k => new Microsoft.IdentityModel.Tokens.RsaSecurityKey(new System.Security.Cryptography.RSAParameters
                {
                    Modulus = Microsoft.IdentityModel.Tokens.Base64UrlEncoder.DecodeBytes(k.N),
                    Exponent = Microsoft.IdentityModel.Tokens.Base64UrlEncoder.DecodeBytes(k.E)
                })).ToList()
            };

            var result = await tokenHandler.ValidateTokenAsync(token, validationParameters);
            if (!result.IsValid)
            {
                _logger.LogError(result.Exception, "JWT token invalid. ValidIssuer={ValidIssuer}", _issuerUrl);
                context.Response.StatusCode = 401;
                context.Response.ContentType = "application/json";
                await context.Response.WriteAsync("Invalid token");
                return;
            }

            if (!string.IsNullOrEmpty(_requiredScopes))
            {
                var required = _requiredScopes.Split(' ', StringSplitOptions.RemoveEmptyEntries);
                var scopeClaim = result.Claims.FirstOrDefault(c => c.Key == "scope");
                if (scopeClaim.Value != null)
                {
                    var scopeStr = scopeClaim.Value.ToString();
                    var actual = scopeStr.Split(' ', StringSplitOptions.RemoveEmptyEntries);
                    foreach (var r in required)
                    {
                        if (!actual.Any(a => a == r))
                        {
                            context.Response.StatusCode = 403;
                            context.Response.ContentType = "application/json";
                            await context.Response.WriteAsync("Insufficient scope");
                            return;
                        }
                    }
                }
            }

            await _next(context);
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "JWT validation failed");
            context.Response.StatusCode = 500;
            context.Response.ContentType = "application/json";
            await context.Response.WriteAsync("Auth failure");
        }
    }

    private async Task<JwkSet> GetJwksAsync()
    {
        if (_cachedJwks != null)
            return _cachedJwks;

        var jwksUrl = $"{_issuerUrl}/.well-known/jwks.json";
        var response = await _httpClient.GetAsync(jwksUrl);
        response.EnsureSuccessStatusCode();
        var json = await response.Content.ReadAsStringAsync();
        var jwks = JsonSerializer.Deserialize<JwkSet>(json, new JsonSerializerOptions { PropertyNameCaseInsensitive = true });
        if (jwks == null)
            throw new InvalidOperationException("Failed to deserialize JWKS");

        _cachedJwks = jwks;
        return jwks;
    }
}

public class JwkSet
{
    public List<JwkKey> Keys { get; set; } = new List<JwkKey>();
}

public class JwkKey
{
    public string Kty { get; set; } = "";
    public string Kid { get; set; } = "";
    public string N { get; set; } = "";
    public string E { get; set; } = "";
    public string Alg { get; set; } = "";
}
