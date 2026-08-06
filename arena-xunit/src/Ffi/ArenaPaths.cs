using System;
using System.IO;
using System.Runtime.InteropServices;

namespace ArenaDotnet.Xunit.Ffi;

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
        if (RuntimeInformation.IsOSPlatform(OSPlatform.OSX))
            return new[] { "libarena_ffi_shared.dylib", "libarena_ffi.dylib" };
        if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
            return new[] { "arena_ffi_shared.dll", "arena_ffi.dll" };
        return new[] { "libarena_ffi_shared.so", "libarena_ffi.so" };
    }
}
