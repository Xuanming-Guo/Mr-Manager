import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { ErrorState } from "../components/AsyncState";
import { PortTable } from "../components/PortTable";
import type { PortEndpoint } from "../types/system";

const endpoints: PortEndpoint[] = [
  {
    protocol: "tcp",
    localAddress: "0.0.0.0",
    localPort: 8080,
    state: "listen",
    owningProcessKey: { pid: 73, startTime: 1_000 },
    owningProcessName: "api-server",
    bindingScope: "allInterfaces",
    inferredScheme: "http",
    localUrl: "http://localhost:8080",
    lanUrls: [],
    reachabilityState: "notTested",
    evidence: ["Windows TCP table"],
  },
  {
    protocol: "tcp",
    localAddress: "127.0.0.1",
    localPort: 5173,
    state: "listen",
    owningProcessKey: { pid: 42, startTime: 900 },
    owningProcessName: "vite",
    bindingScope: "loopback",
    inferredScheme: "http",
    localUrl: "http://localhost:5173",
    lanUrls: [],
    reachabilityState: "notTested",
    evidence: ["Windows TCP table"],
  },
];

describe("PortTable", () => {
  it("filters real endpoints by search, binding, protocol, and common service", async () => {
    const user = userEvent.setup();
    render(<PortTable endpoints={endpoints} />);

    const search = screen.getByPlaceholderText("Search port, address, or owner");
    await user.type(search, "vite");
    expect(screen.getByText("1 endpoints")).toBeInTheDocument();
    expect(screen.queryByText(/api-server/)).not.toBeInTheDocument();

    await user.clear(search);
    await user.selectOptions(screen.getByLabelText("Binding"), "lan");
    expect(screen.getByText(/api-server/)).toBeInTheDocument();
    expect(screen.queryByText(/vite/)).not.toBeInTheDocument();

    await user.selectOptions(screen.getByLabelText("Binding"), "localhost");
    await user.selectOptions(screen.getByLabelText("Protocol"), "tcp");
    await user.selectOptions(screen.getByLabelText("Service"), "common");
    expect(screen.getByText(/vite/)).toBeInTheDocument();
    expect(screen.queryByText(/api-server/)).not.toBeInTheDocument();
  });
});

describe("ErrorState", () => {
  it("announces a typed failure and invokes retry when permitted", async () => {
    const user = userEvent.setup();
    const retry = vi.fn();
    render(
      <ErrorState
        error={{
          code: "COLLECTOR_BUSY",
          message: "Collector is busy.",
          remediation: "Try again.",
          technicalDetails: null,
          retryable: true,
          permissionRelevant: false,
        }}
        retry={retry}
      />,
    );

    expect(screen.getByRole("alert")).toHaveTextContent("COLLECTOR_BUSY");
    await user.click(screen.getByRole("button", { name: "Retry" }));
    expect(retry).toHaveBeenCalledOnce();
  });
});
