using System.Reflection;
using ArenaTools.DotnetXunitRunner;

namespace ArenaExamples.ComponentTest;

public class Program
{
    public static int Main() => XunitConsoleRunner.Run(Assembly.GetExecutingAssembly());
}
