using System;
using System.IO;

namespace ArenaXunit.Ffi;

internal static class ArenaPaths
{
    public static string? ResolveArenaSharedLibrary()
    {
        var envVar = Environment.GetEnvironmentVariable("ARENA_FFI_LIB");
        if (!string.IsNullOrEmpty(envVar) && File.Exists(envVar))
            return envVar;

        var assemblyDir = Path.GetDirectoryName(typeof(ArenaPaths).Assembly.Location);
        if (!string.IsNullOrEmpty(assemblyDir))
        {
            var platformName = PlatformLibraryName();
            var candidate = Path.Combine(assemblyDir, platformName);
            if (File.Exists(candidate))
                return candidate;
        }

        var runfilesRoot = Environment.GetEnvironmentVariable("RUNFILES_DIR");
        if (!string.IsNullOrEmpty(runfilesRoot))
        {
            var paths = new[]
            {
                Path.Combine(runfilesRoot, "arena", "arena-ffi", PlatformLibraryName()),
                Path.Combine(runfilesRoot, "_main", "arena-ffi", PlatformLibraryName()),
            };
            foreach (var path in paths)
            {
                if (File.Exists(path))
                    return path;
            }
        }

        return null;
    }

    private static string PlatformLibraryName()
    {
        var os = Environment.OSVersion.Platform;
        if (os == PlatformID.Unix || os == PlatformID.MacOSX)
        {
            if (os == PlatformID.MacOSX)
                return "libarena_ffi_shared.dylib";
            return "libarena_ffi_shared.so";
        }
        if (os == PlatformID.Win32NT)
            return "arena_ffi_shared.dll";
        return "libarena_ffi_shared.so";
    }
}
