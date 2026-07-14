import { afterEach, describe, expect, it } from "vitest";
import { desktopApi, isDesktopRuntime, normalizeAppError } from "../lib/ipc";
import type { AppError } from "../types/system";

afterEach(() => {
  Reflect.deleteProperty(window, "__TAURI_INTERNALS__");
});

describe("desktop runtime boundary", () => {
  it("detects browser preview and Tauri runtime states", () => {
    expect(isDesktopRuntime()).toBe(false);

    window.__TAURI_INTERNALS__ = {};

    expect(isDesktopRuntime()).toBe(true);
  });

  it("fails closed with a stable error outside Tauri", async () => {
    await expect(desktopApi.getSettings()).rejects.toMatchObject({
      code: "DESKTOP_RUNTIME_REQUIRED",
      retryable: false,
      permissionRelevant: false,
    });
  });
});

describe("normalizeAppError", () => {
  it("preserves typed application errors", () => {
    const error: AppError = {
      code: "ACCESS_DENIED",
      message: "Access denied.",
      remediation: "Review permissions.",
      technicalDetails: null,
      retryable: false,
      permissionRelevant: true,
    };

    expect(normalizeAppError(error)).toBe(error);
  });

  it("converts unknown errors into a safe stable shape", () => {
    expect(normalizeAppError(new Error("collector stopped"))).toEqual({
      code: "UNEXPECTED_ERROR",
      message: "collector stopped",
      remediation: "Retry the operation. If it persists, open Diagnostics.",
      technicalDetails: null,
      retryable: true,
      permissionRelevant: false,
    });
  });
});
