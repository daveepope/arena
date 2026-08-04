using System.Reflection;
using ArenaTools.DotnetXunitRunner;

namespace ArenaDotnet.Xunit.ComponentTest;

public class Program
{
    public static int Main() => XunitConsoleRunner.Run(Assembly.GetExecutingAssembly());
}
