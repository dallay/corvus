import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const exitCallbacks: Array<(code: number | null, signal: NodeJS.Signals | null) => void> = [];
const spawnMock = vi.fn(() => ({
  on: vi.fn(
    (event: string, callback: (code: number | null, signal: NodeJS.Signals | null) => void) => {
      if (event === "exit") {
        exitCallbacks.push(callback);
      }
    }
  ),
}));
const readdirMock = vi.fn();

vi.mock("node:child_process", () => ({
  default: {
    spawn: spawnMock,
  },
  spawn: spawnMock,
}));

vi.mock("node:fs/promises", () => ({
  default: {
    readdir: readdirMock,
  },
  readdir: readdirMock,
}));

describe("dashboard run-coverage script", () => {
  beforeEach(() => {
    vi.resetModules();
    vi.clearAllMocks();
    exitCallbacks.length = 0;
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("spawns vitest with deterministic lcov coverage arguments", async () => {
    readdirMock.mockImplementation(async (dirUrl: URL) => {
      const href = String(dirUrl);
      if (href.includes("/src") && !href.includes("/nested")) {
        return [
          { name: "zeta.spec.ts", isDirectory: () => false, isFile: () => true },
          { name: "alpha.spec.ts", isDirectory: () => false, isFile: () => true },
          { name: "nested", isDirectory: () => true, isFile: () => false },
        ];
      }

      if (href.includes("/nested")) {
        return [
          { name: "beta.spec.ts", isDirectory: () => false, isFile: () => true },
          { name: "notes.txt", isDirectory: () => false, isFile: () => true },
        ];
      }

      return [];
    });

    await import("../../scripts/run-coverage.mjs?sorted-specs");

    expect(spawnMock).toHaveBeenCalledTimes(1);
    const [command, args, options] = spawnMock.mock.calls[0];
    expect(String(command)).toMatch(/pnpm(\.cmd)?$/);
    expect(args.slice(0, 9)).toEqual([
      "exec",
      "vitest",
      "--run",
      "--environment",
      "happy-dom",
      "--coverage",
      "--coverage.reporter=lcov",
      "--coverage.reporter=html",
      "--coverage.reporter=text",
    ]);
    expect(args.slice(9)).toEqual(
      expect.arrayContaining([
        expect.stringMatching(/alpha\.spec\.ts$/),
        expect.stringMatching(/nested[\\/]beta\.spec\.ts$/),
        expect.stringMatching(/zeta\.spec\.ts$/),
      ])
    );
    expect(args.slice(9)).toHaveLength(3);
    expect(String(options.cwd)).toContain("dashboard");
    expect(options.stdio).toBe("inherit");
  });

  it("exits with code 1 when no spec files are found", async () => {
    const exitSpy = vi.spyOn(process, "exit").mockImplementation(((
      code?: string | number | null
    ) => {
      throw new Error(`process.exit:${code ?? "undefined"}`);
    }) as never);
    const errorSpy = vi.spyOn(console, "error").mockImplementation(() => {});
    readdirMock.mockResolvedValue([]);

    await expect(import("../../scripts/run-coverage.mjs?no-specs")).rejects.toThrow(
      "process.exit:1"
    );

    expect(errorSpy).toHaveBeenCalledWith("No dashboard unit spec files were found under src/.");
    expect(exitSpy).toHaveBeenCalledWith(1);
    expect(spawnMock).not.toHaveBeenCalled();
  });

  it("maps child exit outcomes back to the parent process", async () => {
    const exitSpy = vi.spyOn(process, "exit").mockImplementation((() => undefined) as never);
    const killSpy = vi.spyOn(process, "kill").mockImplementation((() => true) as never);
    readdirMock.mockResolvedValue([
      { name: "alpha.spec.ts", isDirectory: () => false, isFile: () => true },
    ]);

    await import("../../scripts/run-coverage.mjs?exit-forwarding");

    expect(exitCallbacks).toHaveLength(1);
    exitCallbacks[0](null, "SIGTERM");
    exitCallbacks[0](0, null);
    exitCallbacks[0](null, null);

    expect(killSpy).toHaveBeenCalledWith(process.pid, "SIGTERM");
    expect(exitSpy).toHaveBeenNthCalledWith(1, 0);
    expect(exitSpy).toHaveBeenNthCalledWith(2, 1);
  });
});
