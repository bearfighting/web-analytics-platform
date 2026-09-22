import { spawn } from "node:child_process";

import { chromium, expect } from "@playwright/test";

const root = new URL("..", import.meta.url).pathname;
const servers = [];

try {
  const react = start("@web-analytics/react-router-playground", 15101);
  const tanstack = start("@web-analytics/tanstack-router-playground", 15102);
  await Promise.all([waitForServer(15101), waitForServer(15102)]);

  const browser = await chromium.launch({ headless: true });
  try {
    await checkRouter(browser, "http://127.0.0.1:15101", "React Router User");
    await checkRouter(browser, "http://127.0.0.1:15102", "TanStack Router User");
  } finally {
    await browser.close();
  }
  console.log("Router adapter E2E passed for React Router and TanStack Router.");
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
} finally {
  for (const server of servers) server.kill("SIGTERM");
}

function start(filter, port) {
  const server = spawn(
    "pnpm",
    ["--filter", filter, "dev", "--host", "127.0.0.1", "--port", String(port)],
    {
      cwd: root,
      stdio: "ignore",
    },
  );
  servers.push(server);
  return server;
}

async function waitForServer(port) {
  const deadline = Date.now() + 30_000;
  while (Date.now() < deadline) {
    try {
      const response = await fetch(`http://127.0.0.1:${port}`);
      if (response.ok) return;
    } catch {
      // The dev server is still starting.
    }
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  throw new Error(`Router playground on port ${port} did not start`);
}

async function checkRouter(browser, baseUrl, expectedTitle) {
  const page = await browser.newPage();
  await page.goto(baseUrl);
  await page.getByRole("link", { name: "User" }).click();
  await page.getByRole("heading", { name: expectedTitle }).waitFor();
  const eventsLocator = page.getByTestId("events");
  await expect(eventsLocator).toContainText('"navigationType":"push"');
  const events = await eventsLocator.textContent();
  if (
    !events?.includes('"navigationType":"initial"') ||
    !events.includes('"navigationType":"push"') ||
    !events.includes("/users/42")
  ) {
    throw new Error(`Navigation event assertion failed for ${baseUrl}: ${events}`);
  }
  await page.getByRole("link", { name: "Search" }).click();
  await page.getByRole("heading", { name: `${expectedTitle.split(" User")[0]} Search` }).waitFor();
  await page.getByRole("button", { name: "Replace" }).click();
  await page.getByRole("heading", { name: expectedTitle }).waitFor();
  await expect(eventsLocator).toContainText('"navigationType":"replace"');

  await page.goBack();
  await expect(eventsLocator).toContainText('"navigationType":"pop"');
  await expect(eventsLocator).toContainText("/users/42");

  const beforeHash = JSON.parse((await eventsLocator.textContent()) ?? "[]").length;
  await page.getByRole("link", { name: "Hash" }).click();
  await page.waitForTimeout(100);
  const afterHash = JSON.parse((await eventsLocator.textContent()) ?? "[]").length;
  if (afterHash !== beforeHash) {
    throw new Error(`Hash-only navigation emitted an event for ${baseUrl}`);
  }

  const analyticsEvents = page.getByTestId("analytics-events");
  await expect(analyticsEvents).toContainText('"type":"page_view"');
  await expect
    .poll(
      async () =>
        ((await analyticsEvents.textContent())?.match(/"type":"page_view"/g) ?? []).length,
    )
    .toBeGreaterThanOrEqual(5);
  await page.close();
}
