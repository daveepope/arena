using System;
using System.IO;
using ArenaXunit.ComponentTest;

var runfilesDir = Environment.GetEnvironmentVariable("RUNFILES_DIR");
if (!string.IsNullOrEmpty(runfilesDir))
{
    var ffiDir = Path.Combine(runfilesDir, "_main", "arena-ffi");
    if (Directory.Exists(ffiDir))
    {
        var ldPath = Environment.GetEnvironmentVariable("LD_LIBRARY_PATH");
        Environment.SetEnvironmentVariable("LD_LIBRARY_PATH",
            string.IsNullOrEmpty(ldPath) ? ffiDir : $"{ffiDir}:{ldPath}");
    }
}

await ArenaLifecycleComponentTest.RunAll();
