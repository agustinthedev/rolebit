export type PageId = "overview" | "opportunities" | "history" | "profile" | "settings";

export interface SearchItem {
  id?: number;
  value: string;
  enabled: boolean;
}

export interface SearchConfiguration {
  sites: SearchItem[];
  locations: SearchItem[];
  pagesPerQuery: number;
}

export interface QueryPreview {
  queries: string[];
  enabledSites: number;
  enabledLocations: number;
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
  hasDecisionsApiKey: boolean;
  databasePath: string;
  appVersion: string;
}

export interface SaveSettingsInput extends Omit<AppSettings, "onboardingComplete" | "hasApiKey" | "hasDecisionsApiKey" | "databasePath" | "appVersion" | "lastScheduledRunAt"> {
  apiKey?: string;
  decisionsApiKey?: string;
}

export interface ProfileExperience {
  id?: number;
  company: string;
  title: string;
  location?: string;
  startDate?: string;
  endDate?: string;
  current: boolean;
  description?: string;
}

export interface ProfileEducation {
  id?: number;
  institution: string;
  degree?: string;
  fieldOfStudy?: string;
  location?: string;
  startDate?: string;
  endDate?: string;
  description?: string;
}

export interface ProfileProject {
  id?: number;
  name: string;
  role?: string;
  url?: string;
  technologies?: string;
  description?: string;
}

export interface ProfileSkill {
  id?: number;
  name: string;
  category?: string;
  level?: string;
}

export interface ProfileLanguage {
  id?: number;
  name: string;
  proficiency?: string;
}

export interface ProfileCertification {
  id?: number;
  name: string;
  issuer?: string;
  issueDate?: string;
  credentialUrl?: string;
}

export interface Profile {
  fullName: string;
  preferredName?: string;
  headline?: string;
  summary?: string;
  email?: string;
  phone?: string;
  location?: string;
  country?: string;
  address?: string;
  linkedinUrl?: string;
  githubUrl?: string;
  portfolioUrl?: string;
  workAuthorization?: string;
  availability?: string;
  experiences: ProfileExperience[];
  education: ProfileEducation[];
  projects: ProfileProject[];
  skills: ProfileSkill[];
  languages: ProfileLanguage[];
  certifications: ProfileCertification[];
  updatedAt?: string;
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
  classificationConfidence?: number | null;
  matchScore?: number | null;
  matchConfidence?: number | null;
  skillsMatch?: number | null;
  experienceMatch?: number | null;
  seniorityMatch?: number | null;
  locationMatch?: number | null;
  matchError?: string | null;
  matchRationale?: string | null;
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
  filteredResults: number;
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
  filteredCount: number;
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
  strongMatches: number;
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
  filteredResults: number;
}

export interface Diagnostics {
  databasePath: string;
  databaseSizeBytes: number;
  sqliteVersion: string;
  appVersion: string;
  schemaVersion: number;
}
