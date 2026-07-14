import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { CleanerPage } from "../routes/CleanerPage";

const mocks = vi.hoisted(() => ({
  executePlan: vi.fn(),
  scanResult: {
    scanId: "scan-1",
    operationId: "operation-1",
    roots: ["C:\\fixture-project"],
    candidates: [
      {
        id: "candidate-1",
        rootPath: "C:\\fixture-project",
        canonicalPath: "C:\\fixture-project\\node_modules",
        displayName: "node_modules",
        category: "dependencyCache",
        reason: "Installed dependencies are reproducible.",
        confidence: "certain",
        risk: "low",
        estimatedSizeBytes: 1024,
        fileCount: 2,
        lockState: "unknown",
        selected: false,
        regenerationInstructions: "Run the locked install command.",
        identityFingerprint: "fixture",
        isDirectory: true,
      },
    ],
    issues: [],
    visitedEntries: 3,
    totalCandidateBytes: 1024,
    cancelled: false,
    limitsReached: false,
  },
}));

vi.mock("../lib/cleaner-ipc", () => ({
  selectCleanupFolders: vi.fn().mockResolvedValue(["C:\\fixture-project"]),
  cleanerApi: {
    manifests: vi.fn().mockResolvedValue([]),
    createPlan: vi.fn().mockResolvedValue({
      id: "plan-1",
      scanId: "scan-1",
      createdAtMs: 1,
      roots: ["C:\\fixture-project"],
      items: [
        {
          id: "candidate-1",
          rootPath: "C:\\fixture-project",
          canonicalPath: "C:\\fixture-project\\node_modules",
          displayName: "node_modules",
          category: "dependencyCache",
          reason: "Installed dependencies are reproducible.",
          confidence: "certain",
          risk: "low",
          estimatedSizeBytes: 1024,
          fileCount: 2,
          lockState: "unknown",
          selected: false,
          regenerationInstructions: "Run the locked install command.",
          identityFingerprint: "fixture",
          isDirectory: true,
        },
      ],
      totalSizeBytes: 1024,
      totalFileCount: 2,
      state: "reviewed",
      confirmationPhrase: "QUARANTINE 1 ITEMS",
      manifestId: null,
    }),
    restore: vi.fn(),
    purge: vi.fn(),
    purgeManifest: vi.fn(),
  },
}));

vi.mock("../lib/tasks-ipc", () => ({
  taskApi: {
    list: vi.fn().mockResolvedValue([]),
    startCleanupScan: vi.fn().mockImplementation(async (request) => ({
      id: request.operationId,
      kind: "cleanupScan",
      label: "Scan selected folders",
      route: "/cleaner",
      state: "succeeded",
      startedAtMs: 1,
      completedAtMs: 2,
      cancellable: true,
      progressPercent: 100,
      summary: "Found 1 reviewable candidate.",
      error: null,
    })),
    get: vi.fn().mockResolvedValue({
      task: { id: "operation-1", kind: "cleanupScan", state: "succeeded" },
      output: { kind: "cleanupScan", value: mocks.scanResult },
    }),
    cancel: vi.fn(),
    startQuarantine: mocks.executePlan,
  },
}));

describe("Cleaner review flow", () => {
  beforeEach(() => {
    mocks.executePlan.mockReset().mockResolvedValue({});
  });

  it("requires candidate review and the exact immutable-plan confirmation", async () => {
    const user = userEvent.setup();
    const client = new QueryClient({
      defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
    });
    render(
      <QueryClientProvider client={client}>
        <CleanerPage />
      </QueryClientProvider>,
    );

    await user.click(screen.getByRole("button", { name: /choose folders/i }));
    await user.click(screen.getByRole("button", { name: /scan selected roots/i }));
    expect(await screen.findByText("node_modules")).toBeInTheDocument();

    const review = screen.getByRole("button", { name: /review plan/i });
    expect(review).toBeDisabled();
    await user.click(screen.getByRole("checkbox"));
    expect(review).toBeEnabled();
    await user.click(review);

    expect(await screen.findByText(/immutable cleanup plan/i)).toBeInTheDocument();
    const execute = screen.getByRole("button", { name: /execute reversible quarantine/i });
    expect(execute).toBeDisabled();
    await user.type(screen.getByRole("textbox"), "QUARANTINE 1 ITEMS");
    expect(execute).toBeEnabled();
    await user.click(execute);

    await waitFor(() =>
      expect(mocks.executePlan).toHaveBeenCalledWith({
        planId: "plan-1",
        confirmation: "QUARANTINE 1 ITEMS",
      }),
    );
  });
});
