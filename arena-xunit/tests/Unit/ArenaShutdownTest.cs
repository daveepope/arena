using System;
using System.Reflection;
using System.Threading;
using Xunit;

namespace ArenaDotnet.Xunit.UnitTest;

public class ArenaShutdownTest
{
    private static Type ShutdownType =>
        typeof(OpenArena).Assembly.GetType("ArenaDotnet.Xunit.ArenaShutdown")
        ?? throw new InvalidOperationException("ArenaShutdown type not found");

    private static object OpenList =>
        ShutdownType
            .GetField("Open", BindingFlags.NonPublic | BindingFlags.Static)!
            .GetValue(null)!;

    private static int TrackedCount() =>
        (int)OpenList.GetType().GetProperty("Count")!.GetValue(OpenList)!;

    [Fact]
    public void Untrack_ArenaNeverTracked_LeavesRegistryUnchanged()
    {
        var before = TrackedCount();

        ShutdownType
            .GetMethod("Untrack", BindingFlags.NonPublic | BindingFlags.Static)!
            .Invoke(null, new object?[] { null });

        Assert.Equal(before, TrackedCount());
    }

    [Fact]
    public void RegisterHooksOnce_CalledRepeatedly_RegistersOnlyOnce()
    {
        var register = ShutdownType.GetMethod(
            "RegisterHooksOnce",
            BindingFlags.NonPublic | BindingFlags.Static)!;
        var flag = ShutdownType.GetField("_hooksRegistered", BindingFlags.NonPublic | BindingFlags.Static)!;

        register.Invoke(null, null);
        register.Invoke(null, null);
        register.Invoke(null, null);

        Assert.Equal(1, (int)flag.GetValue(null)!);
    }

    [Fact]
    public void CloseAll_EmptyRegistry_DoesNotThrow()
    {
        var closeAll = ShutdownType.GetMethod("CloseAll", BindingFlags.NonPublic | BindingFlags.Static)!;
        var gate = ShutdownType.GetField("Gate", BindingFlags.NonPublic | BindingFlags.Static)!.GetValue(null)!;
        var list = (System.Collections.IList)OpenList;

        Monitor.Enter(gate);
        try
        {
            var tracked = new object?[list.Count];
            list.CopyTo(tracked, 0);
            list.Clear();
            try
            {
                var thrown = Record.Exception(() => closeAll.Invoke(null, null));

                Assert.Null(thrown);
            }
            finally
            {
                foreach (var entry in tracked)
                    list.Add(entry);
            }
        }
        finally
        {
            Monitor.Exit(gate);
        }
    }

    [Fact]
    public void Track_RegistryEntries_HoldArenasWeakly()
    {
        var field = ShutdownType.GetField("Open", BindingFlags.NonPublic | BindingFlags.Static)!;

        var elementType = field.FieldType.GetGenericArguments()[0];

        Assert.Equal(typeof(WeakReference<OpenArena>), elementType);
    }
}
