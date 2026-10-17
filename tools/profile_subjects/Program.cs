using System;
using System.Diagnostics;
using System.IO;
using System.Runtime.CompilerServices;
using System.Threading;

internal static class BusyLoop
{
    private const long StepMillis = 10L;

    private static int Main(string[] args)
    {
        long totalMillis = long.Parse(args[0]);
        string marker = args.Length > 1 ? args[1] : null;
        while (true)
        {
            OnePass(totalMillis);
            if (marker == null)
            {
                return 0;
            }

            File.WriteAllBytes(marker, new byte[] { 1 });
        }
    }

    [MethodImpl(MethodImplOptions.NoInlining)]
    private static void OnePass(long totalMillis)
    {
        foo1();
        foo2();
        foo3();
        foo4();
        foo5();
        foo6();
        foo7();
        foo8();
        foo9();
        bar(Math.Max(totalMillis - (StepMillis * 9L), StepMillis));
    }

    [MethodImpl(MethodImplOptions.NoInlining)]
    private static void foo1()
    {
        Stopwatch stopwatch = Stopwatch.StartNew();
        while (stopwatch.ElapsedMilliseconds < StepMillis)
        {
            Thread.SpinWait(1);
        }
    }

    [MethodImpl(MethodImplOptions.NoInlining)]
    private static void foo2()
    {
        Stopwatch stopwatch = Stopwatch.StartNew();
        while (stopwatch.ElapsedMilliseconds < StepMillis)
        {
            Thread.SpinWait(1);
        }
    }

    [MethodImpl(MethodImplOptions.NoInlining)]
    private static void foo3()
    {
        Stopwatch stopwatch = Stopwatch.StartNew();
        while (stopwatch.ElapsedMilliseconds < StepMillis)
        {
            Thread.SpinWait(1);
        }
    }

    [MethodImpl(MethodImplOptions.NoInlining)]
    private static void foo4()
    {
        Stopwatch stopwatch = Stopwatch.StartNew();
        while (stopwatch.ElapsedMilliseconds < StepMillis)
        {
            Thread.SpinWait(1);
        }
    }

    [MethodImpl(MethodImplOptions.NoInlining)]
    private static void foo5()
    {
        Stopwatch stopwatch = Stopwatch.StartNew();
        while (stopwatch.ElapsedMilliseconds < StepMillis)
        {
            Thread.SpinWait(1);
        }
    }

    [MethodImpl(MethodImplOptions.NoInlining)]
    private static void foo6()
    {
        Stopwatch stopwatch = Stopwatch.StartNew();
        while (stopwatch.ElapsedMilliseconds < StepMillis)
        {
            Thread.SpinWait(1);
        }
    }

    [MethodImpl(MethodImplOptions.NoInlining)]
    private static void foo7()
    {
        Stopwatch stopwatch = Stopwatch.StartNew();
        while (stopwatch.ElapsedMilliseconds < StepMillis)
        {
            Thread.SpinWait(1);
        }
    }

    [MethodImpl(MethodImplOptions.NoInlining)]
    private static void foo8()
    {
        Stopwatch stopwatch = Stopwatch.StartNew();
        while (stopwatch.ElapsedMilliseconds < StepMillis)
        {
            Thread.SpinWait(1);
        }
    }

    [MethodImpl(MethodImplOptions.NoInlining)]
    private static void foo9()
    {
        Stopwatch stopwatch = Stopwatch.StartNew();
        while (stopwatch.ElapsedMilliseconds < StepMillis)
        {
            Thread.SpinWait(1);
        }
    }

    [MethodImpl(MethodImplOptions.NoInlining)]
    private static void bar(long millis)
    {
        Stopwatch stopwatch = Stopwatch.StartNew();
        while (stopwatch.ElapsedMilliseconds < millis)
        {
            Thread.SpinWait(1);
        }
    }

}
