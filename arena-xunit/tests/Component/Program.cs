using System.Reflection;
using ArenaTools.DotnetXunitRunner;

namespace ArenaXunit.ComponentTest;

public class Program
{
    public static int Main() => XunitConsoleRunner.Run(Assembly.GetExecutingAssembly());
}
