#nullable enable

using System;
using System.Collections.Generic;
using System.IO;
using System.Reflection;
using System.Security.Cryptography;
using System.Threading;
using Xunit;
using Xunit.Abstractions;

namespace ArenaTools.DotnetXunitRunner;

public static class XunitConsoleRunner
{
    public static int Run(Assembly testAssembly, bool parallelizeTests = false)
    {
        var runfilesRoot = ResolveRunfilesRoot(testAssembly);
        if (!string.IsNullOrEmpty(runfilesRoot))
        {
            var xunitCorePath = FindSingleRunfile(runfilesRoot, "xunit.core.dll");
            if (!string.IsNullOrEmpty(xunitCorePath))
                Assembly.LoadFrom(xunitCorePath);
        }

        var frontController = new XunitFrontController(
            AppDomainSupport.Denied, testAssembly.Location, testAssembly.Location + ".config", false, null, null, null);
        var messageSink = new ConsoleTestMessageSink();
        var options = new XunitRunnerOptions(parallelizeTests);
        frontController.RunAll(messageSink, options, options);

        messageSink.Finished.Wait();
        return messageSink.Failures > 0 ? 1 : 0;
    }

    private static string ResolveRunfilesRoot(Assembly testAssembly)
    {
        var runfilesEnv = Environment.GetEnvironmentVariable("RUNFILES_DIR");
        if (!string.IsNullOrEmpty(runfilesEnv))
            return runfilesEnv;
        var assemblyDir = Path.GetDirectoryName(testAssembly.Location);
        if (!string.IsNullOrEmpty(assemblyDir))
        {
            var parent = Directory.GetParent(assemblyDir)?.FullName;
            if (!string.IsNullOrEmpty(parent) && parent.Contains(".runfiles"))
                return parent;
        }
        return "";
    }

    private static string? FindSingleRunfile(string runfilesRoot, string fileName)
    {
        var matches = Directory.GetFiles(runfilesRoot, fileName, SearchOption.AllDirectories);
        if (matches.Length == 0)
            return null;
        if (matches.Length == 1)
            return matches[0];

        // Bazel runfiles trees commonly expose the same file under more than one path
        // (e.g. a canonical repo-mapped path and an `external/` alias); only treat this
        // as ambiguous if the matches actually have different contents.
        var distinctContents = new HashSet<string>();
        foreach (var match in matches)
            distinctContents.Add(Convert.ToBase64String(SHA256.HashData(File.ReadAllBytes(match))));

        if (distinctContents.Count > 1)
        {
            throw new InvalidOperationException(
                $"ambiguous runfile lookup for '{fileName}' under '{runfilesRoot}': found {matches.Length} matches " +
                $"with different contents ({string.Join(", ", matches)}); narrow the search or reference the file directly");
        }
        return matches[0];
    }
}

internal sealed class ConsoleTestMessageSink : IMessageSink
{
    public int Failures { get; private set; }
    public ManualResetEventSlim Finished { get; } = new ManualResetEventSlim(false);

    public bool OnMessage(IMessageSinkMessage message)
    {
        switch (message)
        {
            case ITestFailed failed:
                Console.WriteLine($"FAILED: {failed.TestCase.DisplayName}");
                foreach (var m in failed.Messages)
                    Console.WriteLine($"  {m}");
                Failures++;
                break;
            case ITestPassed passed:
                Console.WriteLine($"PASSED: {passed.TestCase.DisplayName}");
                break;
            case ITestAssemblyFinished:
                Console.WriteLine($"Total failures: {Failures}");
                Finished.Set();
                break;
        }
        return true;
    }
}

internal sealed class XunitRunnerOptions : ITestFrameworkExecutionOptions, ITestFrameworkDiscoveryOptions
{
    public XunitRunnerOptions(bool parallelizeTests)
    {
        MaxConcurrency = parallelizeTests ? Environment.ProcessorCount : 1;
        ParallelizeClassLevel = parallelizeTests;
        ParallelizeTestLevel = parallelizeTests;
    }

    public int MaxConcurrency { get; }
    public TimeSpan ParallelizeAssemblyLevelTimeout { get; } = TimeSpan.FromSeconds(30);
    public TimeSpan ParallelizeClassLevelTimeout { get; } = TimeSpan.FromSeconds(30);
    public TimeSpan ParallelizeTestLevelTimeout { get; } = TimeSpan.FromSeconds(30);
    public string? AssemblyFileName => null;
    public string? ConfigFileName => null;
    public bool CollectSourceInformation { get; } = false;
    public bool EnsureTestAssemblyLoadedBeforeDiscovery { get; } = true;
    public bool ParallelizeAssemblyLevel { get; } = false;
    public bool ParallelizeClassLevel { get; }
    public bool ParallelizeTestLevel { get; }
    public bool ShouldFailOnNoTests { get; } = true;
    public bool ShouldRunOnRemoteMachine { get; } = false;
    public bool ShadowCopy { get; } = false;
    public bool StopOnFail { get; } = false;
    public string? ShadowCopyFolder => null;
    public bool VerifyTestAssemblyExists { get; } = false;
    public string? DiagnosticOutputFolder => null;
    public string? DiagnosticFileName => null;
    public TValue GetValue<TValue>(string key) => default!;
    public void SetValue<TValue>(string key, TValue value) { }
}
