using System;
using System.Diagnostics;
using System.Security.Cryptography;
using System.Text;
using System.Threading;

namespace ArenaDotnet.Xunit.Support;

public static class ArenaIdentifiers
{
    private static int _counter;
    private static readonly Lazy<long> Seed = new(ComputeSeed, LazyThreadSafetyMode.ExecutionAndPublication);

    private const string ModulePrefix = "arena-";
    private const int SuffixLength = 6;

    public static string Build(string module, string name)
    {
        if (IsAlreadyBuilt(name))
            return name;

        var counter = Interlocked.Increment(ref _counter);
        var slug = ToSlug(name);
        if (string.IsNullOrEmpty(slug))
            slug = "default";
        var combined = Seed.Value + counter;
        var hashBytes = ComputeHash(combined.ToString());
        var suffix = Convert.ToBase64String(hashBytes).Replace("+", "").Replace("/", "").Replace("=", "").ToLowerInvariant();
        var truncatedSuffix = suffix.Length > 6 ? suffix.Substring(0, 6) : suffix.PadRight(6, 'x');
        return $"{module}-{slug}-{truncatedSuffix}";
    }

    private static bool IsAlreadyBuilt(string name)
    {
        if (string.IsNullOrEmpty(name) || !name.StartsWith(ModulePrefix, StringComparison.Ordinal))
            return false;

        var dash = name.LastIndexOf('-');
        if (dash < 0)
            return false;

        var last = name.Substring(dash + 1);
        if (last.Length != SuffixLength)
            return false;

        foreach (var c in last)
        {
            if (!(c >= '0' && c <= '9') && !(c >= 'a' && c <= 'z'))
                return false;
        }

        return true;
    }

    private static byte[] ComputeHash(string input)
    {
        using var sha = SHA256.Create();
        return sha.ComputeHash(Encoding.UTF8.GetBytes(input));
    }

    private static long ComputeSeed()
    {
        var timestamp = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds();
        var processId = (long)Process.GetCurrentProcess().Id;
        var random = new Random();
        return timestamp ^ processId ^ random.Next();
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
