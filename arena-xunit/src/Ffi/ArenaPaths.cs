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

        var viaDepsFile = ResolveViaDepsFile(assemblyDir);
        if (!string.IsNullOrEmpty(viaDepsFile))
            return viaDepsFile;

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

    internal static string? ResolveViaDepsFile(string? assemblyDir)
    {
        var resolverType = Type.GetType(
            "System.Runtime.Loader.AssemblyDependencyResolver, System.Runtime.Loader", throwOnError: false);
        var resolveMethod = resolverType?.GetMethod("ResolveUnmanagedDllToPath", new[] { typeof(string) });
        if (resolverType is null || resolveMethod is null)
            return null;

        if (string.IsNullOrEmpty(assemblyDir) || !Directory.Exists(assemblyDir))
            return null;

        foreach (var depsFile in Directory.EnumerateFiles(assemblyDir, "*.deps.json"))
        {
            try
            {
                var mainAssemblyPath = depsFile.Substring(0, depsFile.Length - ".deps.json".Length) + ".dll";
                var resolver = Activator.CreateInstance(resolverType, mainAssemblyPath);
                foreach (var name in UnmanagedLibraryNames())
                {
                    var resolved = resolveMethod.Invoke(resolver, new object[] { name }) as string;
                    if (!string.IsNullOrEmpty(resolved) && File.Exists(resolved))
                        return resolved;
                }
            }
            catch (Exception)
            {
            }
        }
        return null;
    }

    private static string[] UnmanagedLibraryNames() => new[] { "arena_ffi_shared", "arena_ffi" };

    private static string[] PlatformLibraryNames()
    {
        if (RuntimeInformation.IsOSPlatform(OSPlatform.OSX))
            return new[] { "libarena_ffi_shared.dylib", "libarena_ffi.dylib" };
        if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
            return new[] { "arena_ffi_shared.dll", "arena_ffi.dll" };
        return new[] { "libarena_ffi_shared.so", "libarena_ffi.so" };
    }
}
