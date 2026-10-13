import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { ConfigurationPage } from "./ConfigurationPage";
import type { AppSettings, SearchConfiguration } from "../types";

const configuration: SearchConfiguration = {
  sites: [{ value: "jobs.example.com", enabled: true }],
  locations: [{ value: "Remote", enabled: true }],
  roles: [{ value: "QA", enabled: true }],
  pagesPerQuery: 1,
};

const settings: AppSettings = {
  onboardingComplete: true,
  provider: "mock",
  serpZone: "",
  googleGl: "us",
  googleHl: "en",
  requestConcurrency: 3,
  requestTimeoutSeconds: 30,
  retryCount: 2,
  pagesPerQuery: 1,
  estimatedCostPerRequest: 0.001,
  requestBudget: 100,
  scheduleEnabled: true,
  scheduleIntervalHours: 12,
  runOnStartup: false,
  startWithWindows: false,
  closeToTray: true,
  hasApiKey: false,
  databasePath: "rolebit.sqlite3",
  appVersion: "0.1.0",
};

describe("ConfigurationPage", () => {
  it("blocks ingestion until edited search strategy changes are saved", async () => {
    const user = userEvent.setup();
    const onRun = vi.fn();
    render(<ConfigurationPage initial={configuration} settings={settings} onSaved={vi.fn()} onRun={onRun} />);

    const runButton = screen.getByRole("button", { name: "Run ingestion" });
    await waitFor(() => expect(runButton).toBeEnabled());

    await user.clear(screen.getByLabelText("Roles / Terms item 1"));
    await user.type(screen.getByLabelText("Roles / Terms item 1"), "SDET");
    expect(runButton).toBeDisabled();
    expect(screen.getByText("Save your edited search strategy before starting an ingestion run.")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Save changes" }));
    await waitFor(() => expect(runButton).toBeEnabled());
    expect(onRun).not.toHaveBeenCalled();
  });
});
