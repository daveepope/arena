using System.Reflection;
using ArenaTools.DotnetXunitRunner;

namespace ArenaXunit.UnitTest;

public class Program
{
    public static int Main() => XunitConsoleRunner.Run(Assembly.GetExecutingAssembly(), parallelizeTests: true);
}
