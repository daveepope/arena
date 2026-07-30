using System;
using System.IO;
using System.Reflection;
using System.Linq;
using Xunit;
using Xunit.Abstractions;
using Xunit.Sdk;

namespace ArenaXunit.UnitTest;

public class Program
{
    public static int Main(string[] args)
    {
        var runfilesRoot = GetRunfilesRoot();
        var assembly = Assembly.GetExecutingAssembly();

        if (!string.IsNullOrEmpty(runfilesRoot))
        {
            var xunitCorePath = Directory.GetFiles(runfilesRoot, "xunit.core.dll", SearchOption.AllDirectories).FirstOrDefault();
            if (!string.IsNullOrEmpty(xunitCorePath))
                Assembly.LoadFrom(xunitCorePath);
        }

        var frontController = new XunitFrontController(AppDomainSupport.Denied, assembly.Location, assembly.Location + ".config", false, null, null, null);
        var messageSink = new TestMessageSink();
        var options = new DefaultRunnerReporter(messageSink);
        frontController.RunAll(messageSink, options, options);

        System.Threading.Thread.Sleep(2000);
        return messageSink.Failures > 0 ? 1 : 0;
    }

    private static string GetRunfilesRoot()
    {
        var runfilesEnv = Environment.GetEnvironmentVariable("RUNFILES_DIR");
        if (!string.IsNullOrEmpty(runfilesEnv))
            return runfilesEnv;
        var assemblyDir = Path.GetDirectoryName(Assembly.GetExecutingAssembly().Location);
        if (!string.IsNullOrEmpty(assemblyDir))
        {
            var parent = Directory.GetParent(assemblyDir)?.FullName;
            if (!string.IsNullOrEmpty(parent) && parent.Contains(".runfiles"))
                return parent;
        }
        return "";
    }
}

public class TestMessageSink : IMessageSink
{
    public int Failures { get; private set; }

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
                break;
        }
        return true;
    }
}

public class DefaultRunnerReporter : ITestFrameworkExecutionOptions, ITestFrameworkDiscoveryOptions
{
    private readonly IMessageSink _messageSink;

    public DefaultRunnerReporter(IMessageSink messageSink) => _messageSink = messageSink;

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
