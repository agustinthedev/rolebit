import { describe, expect, it } from "vitest";
import { api } from "./api";
import type { SearchConfiguration } from "../types";

describe("browser preview data boundary", () => {
  it("generates a deterministic deduplicated query preview", async () => {
    const configuration: SearchConfiguration = {
      sites: [
        { value: "https://jobs.example.com/", enabled: true },
        { value: "jobs.example.com", enabled: true },
        { value: "disabled.example.com", enabled: false },
      ],
      locations: [{ value: "Remote", enabled: true }, { value: "", enabled: true }],
      roles: [{ value: "Staff \"QA\"", enabled: true }],
      pagesPerQuery: 2,
    };
    const result = await api.previewQueries({ configuration, scheduleIntervalHours: 12, estimatedCostPerRequest: 0.001 });
    expect(result.queries).toEqual([
      'site:jobs.example.com "Remote" "Staff \\"QA\\""',
      'site:jobs.example.com "Staff \\"QA\\""',
    ]);
    expect(result.totalRequests).toBe(4);
  });

  it("filters, sorts, paginates, and updates tracking state", async () => {
    const page = await api.listJobs({ search: "engineer", trackingStatus: "new", sort: "title", page: 1, pageSize: 25 });
    expect(page.total).toBeGreaterThan(0);
    expect(page.items.every((job) => job.trackingStatus === "new")).toBe(true);
    const updated = await api.updateJobStatus(page.items[0].id, "saved");
    expect(updated.trackingStatus).toBe("saved");
  });

  it("uses the draft schedule interval for monthly preview estimates", async () => {
    const configuration: SearchConfiguration = {
      sites: [{ value: "jobs.example.com", enabled: true }],
      locations: [{ value: "Remote", enabled: true }],
      roles: [{ value: "QA", enabled: true }],
      pagesPerQuery: 1,
    };
    const sixHours = await api.previewQueries({ configuration, scheduleIntervalHours: 6 });
    const twelveHours = await api.previewQueries({ configuration, scheduleIntervalHours: 12 });
    expect(sixHours.estimatedMonthlyRequests).toBe(2 * twelveHours.estimatedMonthlyRequests);
  });
});
