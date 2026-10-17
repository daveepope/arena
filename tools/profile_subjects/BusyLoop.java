import java.nio.file.Files;
import java.nio.file.Path;

public class BusyLoop {
    private static final long STEP_MILLIS = 10L;

    public static void main(String[] args) throws Exception {
        long totalMillis = Long.parseLong(args[0]);
        Path marker = args.length > 1 ? Path.of(args[1]) : null;
        while (true) {
            onePass(totalMillis);
            if (marker == null) {
                return;
            }
            Files.write(marker, new byte[] {1});
        }
    }

    private static void onePass(long totalMillis) {
        foo1();
        foo2();
        foo3();
        foo4();
        foo5();
        foo6();
        foo7();
        foo8();
        foo9();
        bar(Math.max(totalMillis - STEP_MILLIS * 9L, STEP_MILLIS));
    }

    private static void foo1() {
        long end = System.nanoTime() + STEP_MILLIS * 1_000_000L;
        while (System.nanoTime() < end) {
            Thread.onSpinWait();
        }
    }

    private static void foo2() {
        long end = System.nanoTime() + STEP_MILLIS * 1_000_000L;
        while (System.nanoTime() < end) {
            Thread.onSpinWait();
        }
    }

    private static void foo3() {
        long end = System.nanoTime() + STEP_MILLIS * 1_000_000L;
        while (System.nanoTime() < end) {
            Thread.onSpinWait();
        }
    }

    private static void foo4() {
        long end = System.nanoTime() + STEP_MILLIS * 1_000_000L;
        while (System.nanoTime() < end) {
            Thread.onSpinWait();
        }
    }

    private static void foo5() {
        long end = System.nanoTime() + STEP_MILLIS * 1_000_000L;
        while (System.nanoTime() < end) {
            Thread.onSpinWait();
        }
    }

    private static void foo6() {
        long end = System.nanoTime() + STEP_MILLIS * 1_000_000L;
        while (System.nanoTime() < end) {
            Thread.onSpinWait();
        }
    }

    private static void foo7() {
        long end = System.nanoTime() + STEP_MILLIS * 1_000_000L;
        while (System.nanoTime() < end) {
            Thread.onSpinWait();
        }
    }

    private static void foo8() {
        long end = System.nanoTime() + STEP_MILLIS * 1_000_000L;
        while (System.nanoTime() < end) {
            Thread.onSpinWait();
        }
    }

    private static void foo9() {
        long end = System.nanoTime() + STEP_MILLIS * 1_000_000L;
        while (System.nanoTime() < end) {
            Thread.onSpinWait();
        }
    }

    private static void bar(long millis) {
        long end = System.nanoTime() + millis * 1_000_000L;
        while (System.nanoTime() < end) {
            Thread.onSpinWait();
        }
    }

}
