#nullable enable

using System;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Threading;
using Xunit;
using Xunit.Abstractions;

namespace ArenaTools.DotnetXunitRunner;

public static class XunitConsoleRunner
{
    public static int Run(Assembly testAssembly)
    {
        var runfilesRoot = ResolveRunfilesRoot(testAssembly);
        if (!string.IsNullOrEmpty(runfilesRoot))
        {
            var xunitCorePath = Directory.GetFiles(runfilesRoot, "xunit.core.dll", SearchOption.AllDirectories).FirstOrDefault();
            if (!string.IsNullOrEmpty(xunitCorePath))
                Assembly.LoadFrom(xunitCorePath);
        }

        var frontController = new XunitFrontController(
            AppDomainSupport.Denied, testAssembly.Location, testAssembly.Location + ".config", false, null, null, null);
        var messageSink = new ConsoleTestMessageSink();
        var options = new XunitRunnerOptions();
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
    public int MaxConcurrency => 1;
    public TimeSpan ParallelizeAssemblyLevelTimeout { get; } = TimeSpan.FromSeconds(30);
    public TimeSpan ParallelizeClassLevelTimeout { get; } = TimeSpan.FromSeconds(30);
    public TimeSpan ParallelizeTestLevelTimeout { get; } = TimeSpan.FromSeconds(30);
    public string? AssemblyFileName => null;
    public string? ConfigFileName => null;
    public bool CollectSourceInformation { get; } = false;
    public bool EnsureTestAssemblyLoadedBeforeDiscovery { get; } = true;
    public bool ParallelizeAssemblyLevel { get; } = false;
    public bool ParallelizeClassLevel { get; } = false;
    public bool ParallelizeTestLevel { get; } = false;
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
