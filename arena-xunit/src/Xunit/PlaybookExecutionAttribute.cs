using System;
using System.Reflection;
using Xunit.Sdk;

namespace ArenaXunit.Xunit;

[AttributeUsage(AttributeTargets.Assembly)]
public sealed class PlaybookExecutionAttribute : BeforeAfterTestAttribute
{
    public override void Before(MethodInfo methodUnderTest)
    {
        PlaybookScope.BeforeTest(methodUnderTest, methodUnderTest.DeclaringType!);
    }

    public override void After(MethodInfo methodUnderTest)
    {
        PlaybookScope.AfterTest(methodUnderTest, methodUnderTest.DeclaringType!);
    }
}
