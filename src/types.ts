export type PageId = "overview" | "opportunities" | "configuration" | "history" | "settings";

export interface SearchItem {
  id?: number;
  value: string;
  enabled: boolean;
}

export interface SearchConfiguration {
  sites: SearchItem[];
  locations: SearchItem[];
  roles: SearchItem[];
  pagesPerQuery: number;
}

export interface QueryPreview {
  queries: string[];
  enabledSites: number;
  enabledLocations: number;
  enabledRoles: number;
  totalRequests: number;
  estimatedMonthlyRequests: number;
  estimatedCost?: number;
}

export interface QueryPreviewInput {
  configuration: SearchConfiguration;
  scheduleIntervalHours?: number;
  estimatedCostPerRequest?: number | null;
}

export interface AppSettings {
  onboardingComplete: boolean;
  provider: "mock" | "bright_data";
  serpZone: string;
  googleGl: string;
  googleHl: string;
  requestConcurrency: number;
  requestTimeoutSeconds: number;
  retryCount: number;
  pagesPerQuery: number;
  estimatedCostPerRequest?: number;
  requestBudget?: number;
  scheduleEnabled: boolean;
  scheduleIntervalHours: number;
  runOnStartup: boolean;
  startWithWindows: boolean;
  closeToTray: boolean;
  lastScheduledRunAt?: string;
  hasApiKey: boolean;
  databasePath: string;
  appVersion: string;
}

export interface SaveSettingsInput extends Omit<AppSettings, "onboardingComplete" | "hasApiKey" | "databasePath" | "appVersion" | "lastScheduledRunAt"> {
  apiKey?: string;
}

export interface JobSummary {
  id: number;
  title: string;
  company?: string;
  location?: string;
  remoteStatus: string;
  sourceDomain: string;
  lifecycleStatus: string;
  trackingStatus: "new" | "seen" | "saved" | "dismissed";
  extractionStatus: string;
  firstSeenAt: string;
  lastSeenAt: string;
  updatedAt: string;
  contentUpdated: boolean;
}

export interface JobDiscovery {
  query: string;
  provider: string;
  discoveredUrl: string;
  discoveredAt: string;
}

export interface JobDetail extends JobSummary {
  description?: string;
  canonicalUrl: string;
  externalId?: string;
  employmentType?: string;
  publishedAt?: string;
  expiresAt?: string;
  reopened: boolean;
  discoveries: JobDiscovery[];
}

export interface JobListRequest {
  search?: string;
  location?: string;
  remoteStatus?: string;
  source?: string;
  lifecycleStatus?: string;
  trackingStatus?: string;
  discoveredAfter?: string;
  sort: string;
  page: number;
  pageSize: 25 | 50 | 100;
}

export interface JobPage {
  items: JobSummary[];
  total: number;
  page: number;
  pageSize: number;
  facets: { locations: string[]; sources: string[] };
}

export interface IngestionRun {
  id: number;
  startedAt: string;
  completedAt?: string;
  triggerType: string;
  status: string;
  totalQueries: number;
  attemptedRequests: number;
  successfulQueries: number;
  failedQueries: number;
  urlsDiscovered: number;
  newJobs: number;
  updatedJobs: number;
  extractionFailures: number;
  estimatedCost?: number;
  errorSummary?: string;
}

export interface QueryExecution {
  id: number;
  query: string;
  status: string;
  resultCount: number;
  candidateCount: number;
  newJobs: number;
  updatedJobs: number;
  error?: string;
  startedAt: string;
  completedAt?: string;
}

export interface IngestionRunDetail extends IngestionRun {
  queries: QueryExecution[];
}

export interface ActivityDay {
  date: string;
  status: string;
  newJobs: number;
}

export interface Overview {
  totalOpportunities: number;
  newOpportunities: number;
  recentlyUpdated: number;
  activeQueries: number;
  lastIngestionAt?: string;
  nextIngestionAt?: string;
  ingestionStatus: string;
  recentJobs: JobSummary[];
  activity: ActivityDay[];
  latestRun?: IngestionRun;
}

export interface Bootstrap {
  settings: AppSettings;
  configuration: SearchConfiguration;
  overview: Overview;
  recentRuns: IngestionRun[];
}

export interface IngestionProgress {
  runId: number;
  status: string;
  currentQuery?: string;
  completedQueries: number;
  totalQueries: number;
  urlsDiscovered: number;
  newJobs: number;
  updatedJobs: number;
  errors: number;
}

export interface Diagnostics {
  databasePath: string;
  databaseSizeBytes: number;
  sqliteVersion: string;
  appVersion: string;
  schemaVersion: number;
}
