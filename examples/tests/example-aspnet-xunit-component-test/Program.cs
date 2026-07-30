using System;
using System.Reflection;
using Xunit;
using Xunit.Abstractions;

namespace ArenaExamples.ComponentTest;

public class Program
{
    public static int Main(string[] args)
    {
        var assembly = Assembly.GetExecutingAssembly();
        var configFileName = assembly.Location + ".config";
        var frontController = new XunitFrontController(AppDomainSupport.Denied, assembly.Location, configFileName, false, null, null, null);
        
        var messageSink = new TestMessageSink();
        frontController.RunAll(messageSink, null, null);
        
        return messageSink.Failures > 0 ? 1 : 0;
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
                Console.WriteLine($"FAILED: {failed.TestCase.DisplayName}: {failed.Messages[0]}");
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
