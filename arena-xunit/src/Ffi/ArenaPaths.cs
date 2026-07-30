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
            foreach (var name in PlatformLibraryNames())
            {
                var candidate = Path.Combine(assemblyDir, name);
                if (File.Exists(candidate))
                    return candidate;
            }
        }

        var runfilesRoot = Environment.GetEnvironmentVariable("RUNFILES_DIR");
        if (!string.IsNullOrEmpty(runfilesRoot))
        {
            var bases = new[] { "arena/arena-ffi", "_main/arena-ffi" };
            foreach (var basePath in bases)
            {
                foreach (var name in PlatformLibraryNames())
                {
                    var path = Path.Combine(runfilesRoot, basePath, name);
                    if (File.Exists(path))
                        return path;
                }
            }
        }

        return null;
    }

    private static string[] PlatformLibraryNames()
    {
        var os = Environment.OSVersion.Platform;
        if (os == PlatformID.Unix || os == PlatformID.MacOSX)
        {
            if (os == PlatformID.MacOSX)
                return new[] { "libarena_ffi_shared.dylib", "libarena_ffi.dylib" };
            return new[] { "libarena_ffi_shared.so", "libarena_ffi.so" };
        }
        if (os == PlatformID.Win32NT)
            return new[] { "arena_ffi_shared.dll", "arena_ffi.dll" };
        return new[] { "libarena_ffi_shared.so", "libarena_ffi.so" };
    }
}
