import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { SettingsPage } from "../routes/SettingsPage";

vi.mock("../app/queries", () => ({
  useSettingsQuery: () => ({
    data: {
      refreshMode: "normal",
      externalNetworkChecks: false,
      metricHistoryEnabled: false,
      reducedMotion: false,
    },
    error: null,
    isLoading: false,
    refetch: vi.fn(),
  }),
}));

vi.mock("../lib/ipc", () => ({
  desktopApi: { updateSettings: vi.fn() },
}));

describe("Settings refresh modes", () => {
  it("offers only Normal and Fast and omits the retired storage footnote", () => {
    render(
      <QueryClientProvider client={new QueryClient()}>
        <SettingsPage />
      </QueryClientProvider>,
    );

    expect(screen.getByRole("button", { name: /Normal/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Fast/ })).toBeInTheDocument();
    expect(screen.queryByText("Eco")).not.toBeInTheDocument();
    expect(screen.queryByText("Balanced")).not.toBeInTheDocument();
    expect(screen.queryByText("Realtime")).not.toBeInTheDocument();
    expect(screen.queryByText(/never in the public repository/i)).not.toBeInTheDocument();
  });
});
