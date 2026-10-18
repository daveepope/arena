import { spawn, type ChildProcessWithoutNullStreams } from "node:child_process";
import path from "node:path";

const LISTENING_PATTERN = /listening on (http:\/\/127\.0\.0\.1:\d+)/;
const ARENAS_OPEN_LINE = "All arenas open";
const PROCESS_READY_TIMEOUT_MS = 10_000;

export const ORDERS_ARENA = "arena-tea-party";
export const CATALOG_ARENA = "arena-rabbit-hole";
export const FAULTED_ARENA = "arena-looking-glass";

export const PAGE_BACKGROUND = "rgb(14, 19, 48)";

const FAST_STEP_DELAY_MS = 50;
export const NESTED_DEPENDENCY_EDGE_SUFFIX =
  "dependency:queen-of-hearts-db (postgres)>dependency:mad-hatter-service (http)";

function binaryPath(variable: string): string {
  const relative = process.env[variable];
  if (!relative) throw new Error(`${variable} is not set`);
  const runfiles = process.env.JS_BINARY__RUNFILES;
  if (!runfiles) throw new Error("JS_BINARY__RUNFILES is not set");
  return path.join(runfiles, relative);
}

function awaitOutput(
  child: ChildProcessWithoutNullStreams,
  label: string,
  matches: (buffered: string) => string | null,
): Promise<string> {
  return new Promise((resolve, reject) => {
    let buffered = "";
    const timer = setTimeout(() => {
      reject(new Error(`${label} produced no match within ${PROCESS_READY_TIMEOUT_MS}ms:\n${buffered}`));
    }, PROCESS_READY_TIMEOUT_MS);

    const inspect = (chunk: Buffer) => {
      buffered += chunk.toString();
      const found = matches(buffered);
      if (found === null) return;
      clearTimeout(timer);
      resolve(found);
    };

    child.stdout.on("data", inspect);
    child.stderr.on("data", inspect);
    child.on("error", (error) => {
      clearTimeout(timer);
      reject(new Error(`${label} failed to spawn: ${error.message}`));
    });
    child.on("exit", (code) => {
      clearTimeout(timer);
      reject(new Error(`${label} exited with ${code} before becoming ready:\n${buffered}`));
    });
  });
}

export function stopProcess(child: ChildProcessWithoutNullStreams | null): void {
  if (child === null || child.exitCode !== null) return;
  child.kill("SIGKILL");
}

export async function startDashboardServer(): Promise<{ child: ChildProcessWithoutNullStreams; url: string }> {
  const child = spawn(binaryPath("ARENA_DASHBOARD_SERVER_BIN"), ["0"], { stdio: "pipe" });
  try {
    const url = await awaitOutput(child, "dashboard server", (buffered) => {
      const found = buffered.match(LISTENING_PATTERN);
      return found ? found[1] : null;
    });
    return { child, url };
  } catch (error) {
    stopProcess(child);
    throw error;
  }
}

export async function startDemoLifecycle(dashboardUrl: string): Promise<ChildProcessWithoutNullStreams> {
  const child = spawn(binaryPath("ARENA_DEMO_LIFECYCLE_BIN"), [FAST_STEP_DELAY_MS.toString()], {
    stdio: "pipe",
    env: { ...process.env, ARENA_DASHBOARD_URL: dashboardUrl },
  });
  try {
    await awaitOutput(child, "demo lifecycle", (buffered) =>
      buffered.includes(ARENAS_OPEN_LINE) ? ARENAS_OPEN_LINE : null,
    );
    return child;
  } catch (error) {
    stopProcess(child);
    throw error;
  }
}

export type ArenaStack = {
  server: ChildProcessWithoutNullStreams;
  demo: ChildProcessWithoutNullStreams;
  url: string;
};

export async function startArenaStack(): Promise<ArenaStack> {
  const { child: server, url } = await startDashboardServer();
  try {
    const demo = await startDemoLifecycle(url);
    return { server, demo, url };
  } catch (error) {
    stopProcess(server);
    throw error;
  }
}

export function stopArenaStack(stack: ArenaStack | null): void {
  if (stack === null) return;
  stopProcess(stack.demo);
  stopProcess(stack.server);
}
