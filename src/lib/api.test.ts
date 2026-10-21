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
      pagesPerQuery: 2,
    };
    const result = await api.previewQueries({ configuration, scheduleIntervalHours: 12, estimatedCostPerRequest: 0.001 });
    expect(result.queries).toEqual([
      'site:jobs.example.com "Remote"',
      'site:jobs.example.com',
    ]);
    expect(result.totalRequests).toBe(4);
  });

  it("preserves configured site paths in the query preview", async () => {
    const configuration: SearchConfiguration = {
      sites: [
        { value: "https://job-boards.greenhouse.io/*/jobs/*/", enabled: true },
        { value: "https://apply.workable.com/*/j/*/", enabled: true },
      ],
      locations: [{ value: "Uruguay", enabled: true }],
      pagesPerQuery: 1,
    };
    const result = await api.previewQueries({ configuration });
    expect(result.queries).toEqual([
      'site:job-boards.greenhouse.io/*/jobs/* "Uruguay"',
      'site:apply.workable.com/*/j/* "Uruguay"',
    ]);
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
      pagesPerQuery: 1,
    };
    const sixHours = await api.previewQueries({ configuration, scheduleIntervalHours: 6 });
    const twelveHours = await api.previewQueries({ configuration, scheduleIntervalHours: 12 });
    expect(sixHours.estimatedMonthlyRequests).toBe(2 * twelveHours.estimatedMonthlyRequests);
  });

  it("stores profile data in the local preview boundary", async () => {
    const profile = await api.saveProfile({
      fullName: "Ada Lovelace",
      headline: "Analytical engineer",
      experiences: [{ company: "Example Labs", title: "Staff Engineer", current: true }],
      education: [],
      projects: [{ name: "Rolebit", technologies: "Rust, React" }],
      skills: [{ name: "Rust", category: "Languages" }],
      languages: [{ name: "English", proficiency: "professional" }],
      certifications: [],
    });
    expect(profile.fullName).toBe("Ada Lovelace");
    expect((await api.profile()).experiences[0].company).toBe("Example Labs");
    expect((await api.profile()).projects[0].name).toBe("Rolebit");
    expect((await api.profile()).languages[0].proficiency).toBe("professional");
  });

  it("rejects incomplete profile entries before replacing saved data", async () => {
    const current = await api.profile();
    await expect(api.saveProfile({
      ...current,
      education: [{ institution: "", degree: "Computer Science" }],
    })).rejects.toBe("Education entry 1 needs an institution.");
    expect((await api.profile()).fullName).toBe("Ada Lovelace");
  });
});
