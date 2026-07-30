using System;
using System.Diagnostics;
using System.Security.Cryptography;
using System.Text;

namespace ArenaXunit.Support;

public static class ArenaIdentifiers
{
    private static readonly object Lock = new object();
    private static int _counter;
    private static long? _seed;

    public static string Build(string module, string name)
    {
        var seed = InitializeSeed();
        lock (Lock)
        {
            _counter++;
        }
        var slug = ToSlug(name);
        if (string.IsNullOrEmpty(slug))
            slug = "default";
        var combined = seed + _counter;
        var hashBytes = ComputeHash(combined.ToString());
        var suffix = Convert.ToBase64String(hashBytes).Replace("+", "").Replace("/", "").Replace("=", "").ToLower();
        var truncatedSuffix = suffix.Length > 6 ? suffix.Substring(0, 6) : suffix.PadRight(6, 'x');
        return $"{module}-{slug}-{truncatedSuffix}";
    }

    private static byte[] ComputeHash(string input)
    {
        using var sha = SHA256.Create();
        return sha.ComputeHash(Encoding.UTF8.GetBytes(input));
    }

    private static long InitializeSeed()
    {
        if (_seed.HasValue)
            return _seed.Value;

        lock (Lock)
        {
            if (!_seed.HasValue)
            {
                var timestamp = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();
                var processId = (long)Process.GetCurrentProcess().Id;
                var random = new Random();
                _seed = timestamp ^ processId ^ random.Next();
            }
        }
        return _seed.Value;
    }

    private static string ToSlug(string name)
    {
        var builder = new StringBuilder(name.Length);
        foreach (var c in name.ToLowerInvariant())
        {
            if (char.IsLetterOrDigit(c))
                builder.Append(c);
            else if (builder.Length > 0 && !char.IsWhiteSpace(builder[builder.Length - 1]))
                builder.Append('-');
        }
        if (builder.Length > 0 && builder[builder.Length - 1] == '-')
            builder.Length--;
        return builder.ToString();
    }
}
