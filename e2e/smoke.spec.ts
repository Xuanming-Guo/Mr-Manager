import { expect, test } from "@playwright/test";

test("web preview loads honestly without external network requests", async ({ page }) => {
  const requestedHosts = new Set<string>();

  page.on("request", (request) => {
    requestedHosts.add(new URL(request.url()).hostname);
  });

  await page.goto("/");

  await expect(page.getByText("Mr Manager", { exact: true })).toBeVisible();
  await expect(page.getByRole("navigation", { name: "Primary navigation" })).toBeVisible();
  await expect(page.getByText(/system data unavailable/i)).toBeVisible();
  await expect(
    page.getByText("Real system data is available only in the Mr Manager application."),
  ).toBeVisible();

  expect(requestedHosts.size).toBeGreaterThan(0);
  expect([...requestedHosts].every((host) => host === "127.0.0.1" || host === "localhost")).toBe(
    true,
  );
});

test("legacy diagnostics route redirects to System Diagnostics", async ({ page }) => {
  await page.goto("/#/war-room");

  await expect(page).toHaveURL(/#\/system-diagnostics$/);
  await expect(page.getByRole("link", { name: "System Diagnostics" })).toHaveAttribute(
    "aria-current",
    "page",
  );
});
