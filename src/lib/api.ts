import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import { validateProfile } from "./profile";
import type {
  AppSettings,
  Bootstrap,
  Diagnostics,
  IngestionProgress,
  IngestionRun,
  IngestionRunDetail,
  JobDetail,
  JobListRequest,
  JobPage,
  Profile,
  QueryPreview,
  QueryPreviewInput,
  SaveSettingsInput,
  SearchConfiguration,
} from "../types";

declare global {
  interface Window { __TAURI_INTERNALS__?: unknown }
}

export const isDesktop = () => Boolean(window.__TAURI_INTERNALS__);

const now = new Date();
const hoursAgo = (hours: number) => new Date(now.getTime() - hours * 3_600_000).toISOString();

let demoConfig: SearchConfiguration = {
  sites: ["jobs.ashbyhq.com", "jobs.lever.co", "job-boards.greenhouse.io"].map((value, index) => ({ id: index + 1, value, enabled: true })),
  locations: ["Uruguay", "LATAM", "Remote", "Worldwide"].map((value, index) => ({ id: index + 10, value, enabled: true })),
  pagesPerQuery: 1,
};

let demoSettings: AppSettings = {
  onboardingComplete: !new URLSearchParams(window.location.search).has("onboarding"),
  provider: "mock",
  serpZone: "",
  googleGl: "uy",
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
  lastScheduledRunAt: hoursAgo(7),
  hasApiKey: false,
  hasDecisionsApiKey: false,
  databasePath: "C:\\Users\\You\\AppData\\Roaming\\com.rolebit.desktop\\rolebit.sqlite3",
  appVersion: "0.1.0",
};

let demoProfile: Profile = {
  fullName: "",
  experiences: [],
  education: [],
  projects: [],
  skills: [],
  languages: [],
  certifications: [],
};

const demoJobs: JobDetail[] = [
  ["Senior QA Engineer", "Northstar Labs", "Remote · LATAM", "remote", "jobs.ashbyhq.com", "saved"],
  ["Software Engineer, Developer Experience", "Mosaic Works", "Montevideo, Uruguay", "hybrid", "jobs.lever.co", "new"],
  ["Staff SDET", "Orbitline", "Americas", "remote", "job-boards.greenhouse.io", "new"],
  ["DevOps Engineer", "Luma Systems", "Remote", "remote", "jobs.ashbyhq.com", "seen"],
  ["Product Engineer", "Cedar & Finch", "Worldwide", "remote", "jobs.lever.co", "saved"],
  ["Quality Engineering Lead", "Velora", "Buenos Aires", "hybrid", "job-boards.greenhouse.io", "new"],
  ["Platform Engineer", "Fieldwork", "São Paulo", "onsite", "jobs.ashbyhq.com", "seen"],
  ["Senior Product Manager", "Tandem Cloud", "LATAM", "remote", "jobs.lever.co", "dismissed"],
].map(([title, company, location, remoteStatus, sourceDomain, trackingStatus], index) => ({
  id: index + 1,
  title,
  company,
  location,
  remoteStatus,
  sourceDomain,
  lifecycleStatus: "active",
  trackingStatus: trackingStatus as JobDetail["trackingStatus"],
  extractionStatus: index % 3 === 0 ? "html_fallback" : "json_ld",
  firstSeenAt: hoursAgo(index * 15 + 2),
  lastSeenAt: hoursAgo(index + 1),
  updatedAt: hoursAgo(index * 3 + 1),
  contentUpdated: index === 1 || index === 4,
  description: `${company} is looking for a ${title} to join a thoughtful, distributed product team. You will improve reliable delivery, partner across engineering and product, and shape practical systems that make excellent work easier. This is local browser-preview data; desktop ingestion stores verified postings in SQLite.`,
  canonicalUrl: `https://${sourceDomain}/rolebit-demo/${index + 1}`,
  externalId: `RB-${1000 + index}`,
  employmentType: "Full-time",
  publishedAt: hoursAgo(index * 24 + 24),
  reopened: false,
  discoveries: [
    { query: `site:${sourceDomain} "${location}" "${title.split(",")[0]}"`, provider: "mock", discoveredUrl: `https://${sourceDomain}/rolebit-demo/${index + 1}`, discoveredAt: hoursAgo(index * 15 + 2) },
  ],
}));

let demoRuns: IngestionRun[] = [
  { id: 3, startedAt: hoursAgo(7), completedAt: hoursAgo(6.8), triggerType: "scheduled", status: "completed", totalQueries: 12, attemptedRequests: 12, successfulQueries: 12, failedQueries: 0, urlsDiscovered: 47, filteredResults: 2, newJobs: 8, updatedJobs: 2, extractionFailures: 3, estimatedCost: 0.012 },
  { id: 2, startedAt: hoursAgo(31), completedAt: hoursAgo(30.7), triggerType: "manual", status: "partial", totalQueries: 12, attemptedRequests: 12, successfulQueries: 11, failedQueries: 1, urlsDiscovered: 39, filteredResults: 8, newJobs: 5, updatedJobs: 1, extractionFailures: 4, estimatedCost: 0.012, errorSummary: "One query was rate limited after retries." },
  { id: 1, startedAt: hoursAgo(55), completedAt: hoursAgo(54.8), triggerType: "manual", status: "completed", totalQueries: 12, attemptedRequests: 12, successfulQueries: 12, failedQueries: 0, urlsDiscovered: 33, filteredResults: 1, newJobs: 12, updatedJobs: 0, extractionFailures: 2, estimatedCost: 0.012 },
];

let progressListener: ((progress: IngestionProgress) => void) | undefined;

function generatedQueries(configuration: SearchConfiguration): string[] {
  const values = (items: SearchConfiguration["sites"]) => items.filter((item) => item.enabled);
  const seen = new Set<string>();
  for (const site of values(configuration.sites)) {
    const domain = site.value.trim().replace(/^https?:\/\//, "").replace(/\/$/, "").split("/")[0].toLowerCase();
    if (!domain) continue;
    for (const location of values(configuration.locations)) {
      const quote = (value: string) => `"${value.trim().replaceAll("\\", "\\\\").replaceAll('"', '\\"')}"`;
      seen.add([`site:${domain}`, location.value.trim() && quote(location.value)].filter(Boolean).join(" "));
    }
  }
  return [...seen];
}

function preview(input: QueryPreviewInput): QueryPreview {
  const { configuration } = input;
  const queries = generatedQueries(configuration);
  const totalRequests = queries.length * Math.max(1, configuration.pagesPerQuery);
  const scheduleIntervalHours = input.scheduleIntervalHours ?? demoSettings.scheduleIntervalHours;
  const costPerRequest = input.estimatedCostPerRequest === undefined ? demoSettings.estimatedCostPerRequest : input.estimatedCostPerRequest;
  const perMonth = scheduleIntervalHours ? Math.floor(720 / scheduleIntervalHours) : 0;
  return {
    queries,
    enabledSites: configuration.sites.filter((item) => item.enabled).length,
    enabledLocations: configuration.locations.filter((item) => item.enabled).length,
    totalRequests,
    estimatedMonthlyRequests: totalRequests * perMonth,
    estimatedCost: costPerRequest === undefined || costPerRequest === null ? undefined : totalRequests * costPerRequest,
  };
}

function overview() {
  const latest = demoRuns[0];
  return {
    totalOpportunities: demoJobs.length,
    newOpportunities: demoJobs.filter((job) => job.trackingStatus === "new").length,
    recentlyUpdated: demoJobs.filter((job) => job.contentUpdated).length,
    activeQueries: generatedQueries(demoConfig).length,
    lastIngestionAt: latest?.completedAt,
    nextIngestionAt: demoSettings.scheduleEnabled ? new Date(new Date(latest.completedAt!).getTime() + demoSettings.scheduleIntervalHours * 3_600_000).toISOString() : undefined,
    ingestionStatus: latest?.status ?? "idle",
    recentJobs: demoJobs.slice(0, 5),
    activity: Array.from({ length: 28 }, (_, index) => ({ date: hoursAgo((27 - index) * 24), status: index % 8 === 0 ? "partial" : index % 3 === 0 ? "completed" : "empty", newJobs: index % 3 === 0 ? (index % 5) + 1 : 0 })),
    latestRun: latest,
  };
}

async function demoInvoke<T>(command: string, args: Record<string, unknown> = {}): Promise<T> {
  await new Promise((resolve) => setTimeout(resolve, 80));
  switch (command) {
    case "get_bootstrap": return { settings: demoSettings, configuration: demoConfig, overview: overview(), recentRuns: demoRuns } as T;
    case "get_overview": return overview() as T;
    case "get_search_configuration": return demoConfig as T;
    case "save_search_configuration": demoConfig = args.configuration as SearchConfiguration; return demoConfig as T;
    case "get_profile": return demoProfile as T;
    case "save_profile": demoProfile = args.profile as Profile; return demoProfile as T;
    case "preview_queries": return preview(args.input as QueryPreviewInput) as T;
    case "get_settings": return demoSettings as T;
    case "save_settings": {
      const input = args.settings as SaveSettingsInput;
      demoSettings = { ...demoSettings, ...input, hasApiKey: Boolean(input.apiKey || demoSettings.hasApiKey), hasDecisionsApiKey: Boolean(input.decisionsApiKey || demoSettings.hasDecisionsApiKey) };
      return demoSettings as T;
    }
    case "complete_onboarding": {
      const input = args.input as { settings: SaveSettingsInput; configuration: SearchConfiguration };
      demoConfig = input.configuration;
      demoSettings = { ...demoSettings, ...input.settings, onboardingComplete: true, hasApiKey: Boolean(input.settings.apiKey), hasDecisionsApiKey: Boolean(input.settings.decisionsApiKey) };
      return { settings: demoSettings, configuration: demoConfig, overview: overview(), recentRuns: demoRuns } as T;
    }
    case "list_jobs": return listDemoJobs(args.request as JobListRequest) as T;
    case "get_job": return demoJobs.find((job) => job.id === args.id) as T;
    case "update_tracking_status": {
      const job = demoJobs.find((item) => item.id === args.id)!;
      job.trackingStatus = args.status as JobDetail["trackingStatus"];
      return job as T;
    }
    case "list_ingestion_runs": return demoRuns as T;
    case "get_ingestion_run": {
      const run = demoRuns.find((item) => item.id === args.id)!;
      return { ...run, queries: Array.from({ length: 8 }, (_, index) => ({ id: index + 1, query: generatedQueries(demoConfig)[index], status: index === 7 && run.status === "partial" ? "failed" : "completed", resultCount: 8 - (index % 3), candidateCount: 5, filteredCount: index % 4 === 0 ? 1 : 0, newJobs: index % 3 === 0 ? 1 : 0, updatedJobs: index === 2 ? 1 : 0, error: index === 7 && run.status === "partial" ? "Rate limited after retries" : undefined, startedAt: run.startedAt, completedAt: run.completedAt })) } as T;
    }
    case "get_diagnostics": return { databasePath: demoSettings.databasePath, databaseSizeBytes: 482_304, sqliteVersion: "3.50.4", appVersion: demoSettings.appVersion, schemaVersion: 1 } as T;
    case "test_bright_data_connection": return "Browser preview cannot make paid requests. Run the desktop app to test this connection." as T;
    case "test_decisions_connection": return "Browser preview cannot make API requests. Run the desktop app to test this connection." as T;
    case "start_ingestion": return simulateRun() as T;
    case "cancel_ingestion": return undefined as T;
    default: throw new Error(`Browser preview does not implement ${command}`);
  }
}

function listDemoJobs(request: JobListRequest): JobPage {
  let items = demoJobs.filter((job) => {
    const term = request.search?.toLowerCase();
    return (!term || `${job.title} ${job.company} ${job.description}`.toLowerCase().includes(term))
      && (!request.location || request.location === "all" || job.location === request.location)
      && (!request.remoteStatus || request.remoteStatus === "all" || job.remoteStatus === request.remoteStatus)
      && (!request.source || request.source === "all" || job.sourceDomain === request.source)
      && (!request.lifecycleStatus || request.lifecycleStatus === "all" || job.lifecycleStatus === request.lifecycleStatus)
      && (!request.trackingStatus || request.trackingStatus === "all" || job.trackingStatus === request.trackingStatus);
  });
  items = [...items].sort((a, b) => {
    if (request.sort === "title") return a.title.localeCompare(b.title);
    const left = request.sort === "updated" ? a.updatedAt : a.firstSeenAt;
    const right = request.sort === "updated" ? b.updatedAt : b.firstSeenAt;
    return request.sort === "oldest" ? left.localeCompare(right) : right.localeCompare(left);
  });
  const start = (request.page - 1) * request.pageSize;
  return {
    items: items.slice(start, start + request.pageSize), total: items.length, page: request.page, pageSize: request.pageSize,
    facets: { locations: [...new Set(demoJobs.map((job) => job.location).filter(Boolean) as string[])], sources: [...new Set(demoJobs.map((job) => job.sourceDomain))] },
  };
}

function simulateRun(): number {
  const id = Math.max(...demoRuns.map((run) => run.id)) + 1;
  const total = generatedQueries(demoConfig).length;
  const startedAt = new Date().toISOString();
  const run: IngestionRun = { id, startedAt, triggerType: "manual", status: "running", totalQueries: total, attemptedRequests: 0, successfulQueries: 0, failedQueries: 0, urlsDiscovered: 0, filteredResults: 0, newJobs: 0, updatedJobs: 0, extractionFailures: 0 };
  demoRuns = [run, ...demoRuns];
  let completed = 0;
  const timer = window.setInterval(() => {
    completed += Math.max(1, Math.ceil(total / 12));
    if (completed >= total) {
      completed = total;
      window.clearInterval(timer);
      Object.assign(run, { status: "completed", completedAt: new Date().toISOString(), attemptedRequests: total, successfulQueries: total, urlsDiscovered: Math.ceil(total * 0.65), newJobs: 3, updatedJobs: 1 });
    }
    progressListener?.({ runId: id, status: run.status, currentQuery: run.status === "running" ? generatedQueries(demoConfig)[Math.min(completed, total - 1)] : undefined, completedQueries: completed, totalQueries: total, urlsDiscovered: Math.ceil(completed * 0.65), newJobs: run.status === "completed" ? 3 : Math.floor(completed / 18), updatedJobs: run.status === "completed" ? 1 : 0, errors: 0, filteredResults: 0 });
  }, 180);
  return id;
}

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  return isDesktop() ? invoke<T>(command, args) : demoInvoke<T>(command, args);
}

export const api = {
  bootstrap: () => call<Bootstrap>("get_bootstrap"),
  overview: () => call<Bootstrap["overview"]>("get_overview"),
  configuration: () => call<SearchConfiguration>("get_search_configuration"),
  saveConfiguration: (configuration: SearchConfiguration) => call<SearchConfiguration>("save_search_configuration", { configuration }),
  profile: () => call<Profile>("get_profile"),
  saveProfile: (profile: Profile) => {
    const validationError = validateProfile(profile);
    return validationError ? Promise.reject(validationError) : call<Profile>("save_profile", { profile });
  },
  previewQueries: (input: QueryPreviewInput) => call<QueryPreview>("preview_queries", { input }),
  settings: () => call<AppSettings>("get_settings"),
  saveSettings: (settings: SaveSettingsInput) => call<AppSettings>("save_settings", { settings }),
  completeOnboarding: (input: { settings: SaveSettingsInput; configuration: SearchConfiguration }) => call<Bootstrap>("complete_onboarding", { input }),
  testConnection: (apiKey: string | undefined, zone: string) => call<string>("test_bright_data_connection", { apiKey, zone }),
  testDecisionsConnection: (apiKey?: string) => call<string>("test_decisions_connection", { apiKey }),
  listJobs: (request: JobListRequest) => call<JobPage>("list_jobs", { request }),
  job: (id: number) => call<JobDetail>("get_job", { id }),
  updateJobStatus: (id: number, status: JobDetail["trackingStatus"]) => call<JobDetail>("update_tracking_status", { id, status }),
  runs: (limit = 100) => call<IngestionRun[]>("list_ingestion_runs", { limit }),
  run: (id: number) => call<IngestionRunDetail>("get_ingestion_run", { id }),
  diagnostics: () => call<Diagnostics>("get_diagnostics"),
  startIngestion: () => call<number>("start_ingestion"),
  cancelIngestion: () => call<void>("cancel_ingestion"),
  openExternal: (url: string) => isDesktop() ? openUrl(url) : Promise.resolve(window.open(url, "_blank", "noopener,noreferrer")).then(() => undefined),
  onProgress: async (callback: (progress: IngestionProgress) => void): Promise<UnlistenFn> => {
    if (isDesktop()) return listen<IngestionProgress>("ingestion-progress", (event) => callback(event.payload));
    progressListener = callback;
    return () => { progressListener = undefined; };
  },
};
