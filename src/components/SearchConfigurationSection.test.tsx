import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { SearchConfigurationSection } from "./SearchConfigurationSection";
import type { AppSettings, SearchConfiguration } from "../types";

const configuration: SearchConfiguration = {
  sites: [{ value: "jobs.example.com", enabled: true }],
  locations: [{ value: "Remote", enabled: true }],
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
  hasDecisionsApiKey: false,
  databasePath: "rolebit.sqlite3",
  appVersion: "0.1.0",
};

describe("SearchConfigurationSection", () => {
  it("keeps the search plan visible without an ingestion action", () => {
    render(<SearchConfigurationSection initial={configuration} settings={settings} onSaved={vi.fn()} />);
    expect(screen.getByText("Planned requests")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Run ingestion" })).not.toBeInTheDocument();
  });
});
