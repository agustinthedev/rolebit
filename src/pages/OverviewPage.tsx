import { ArrowRight, BriefcaseBusiness, CalendarClock, CircleCheck, Clock3, RefreshCw, Search, Sparkles, TrendingUp, X } from "lucide-react";
import { compactNumber, formatDate, timeAgo } from "../lib/format";
import type { IngestionProgress, Overview, PageId } from "../types";
import { Button, EmptyState, PageHeader, StatusBadge, SurfaceCard, TechLabel } from "../components/ui";

export function OverviewPage({ overview, progress, onStart, onCancel, onNavigate, onSelectJob, busy }: {
  overview: Overview;
  progress?: IngestionProgress;
  onStart: () => void;
  onCancel: () => void;
  onNavigate: (page: PageId) => void;
  onSelectJob: (id: number) => void;
  busy: boolean;
}) {
  const running = progress?.status === "running" || overview.ingestionStatus === "running";
  return <div className="page"><PageHeader eyebrow="Opportunity command center" title="Good morning." description="A calm view of what Rolebit has found, changed, and scheduled next." actions={running ? <Button variant="secondary" icon={X} onClick={onCancel}>Cancel run</Button> : <Button variant="primary" icon={RefreshCw} disabled={busy} onClick={onStart}>{busy ? "Starting…" : "Run ingestion"}</Button>} /><section className="metric-grid"><Metric label="Total opportunities" value={overview.totalOpportunities} icon={BriefcaseBusiness} tone="purple" note="Stored locally" /><Metric label="New opportunities" value={overview.newOpportunities} icon={Sparkles} tone="mint" note="Ready to review" /><Metric label="Recently updated" value={overview.recentlyUpdated} icon={TrendingUp} tone="coral" note="Meaningful changes" /><Metric label="Active queries" value={overview.activeQueries} icon={Search} tone="blue" note="Current strategy" /></section>{running && progress && <SurfaceCard className="ingestion-live"><div className="ingestion-live__header"><div><TechLabel>Live ingestion</TechLabel><h3>Scanning your opportunity landscape</h3></div><span>{progress.completedQueries} / {progress.totalQueries}</span></div><div className="progress-track"><span style={{ width: `${Math.max(2, progress.totalQueries ? progress.completedQueries / progress.totalQueries * 100 : 2)}%` }} /></div><div className="ingestion-live__stats"><span><strong>{progress.urlsDiscovered}</strong> URLs discovered</span><span><strong>{progress.newJobs}</strong> new jobs</span><span><strong>{progress.updatedJobs}</strong> updated</span><span><strong>{progress.errors}</strong> errors</span></div><p>{progress.currentQuery ?? "Preparing queries…"}</p></SurfaceCard>}<section className="overview-layout"><SurfaceCard className="recent-panel"><div className="panel-heading"><div><TechLabel>Recent opportunities</TechLabel><h2>Fresh from your searches</h2></div><Button variant="ghost" icon={ArrowRight} onClick={() => onNavigate("opportunities")}>View all</Button></div>{overview.recentJobs.length ? <div className="opportunity-list">{overview.recentJobs.map((job) => <button className="opportunity-row" key={job.id} onClick={() => onSelectJob(job.id)}><span className="company-avatar">{(job.company ?? job.title).slice(0, 2).toUpperCase()}</span><span className="opportunity-row__main"><strong>{job.title}</strong><small>{job.company ?? "Company unknown"} · {job.location ?? "Location unknown"}</small></span><StatusBadge value={job.trackingStatus} /><span className="opportunity-row__date">{timeAgo(job.firstSeenAt)}</span><ArrowRight size={16} /></button>)}</div> : <EmptyState icon={BriefcaseBusiness} title="Your library is waiting" description="Configure a search strategy, then run your first ingestion. No paid searches start automatically." action={<Button variant="primary" icon={RefreshCw} onClick={onStart}>Run first ingestion</Button>} />}</SurfaceCard><div className="overview-side"><SurfaceCard className="status-panel"><div className="panel-heading"><div><TechLabel>Ingestion status</TechLabel><h2>Search rhythm</h2></div><StatusBadge value={running ? "running" : overview.ingestionStatus} /></div><div className="status-timeline"><div><span><CircleCheck size={17} /></span><div><small>Last ingestion</small><strong>{timeAgo(overview.lastIngestionAt)}</strong><p>{formatDate(overview.lastIngestionAt, true)}</p></div></div><div><span><CalendarClock size={17} /></span><div><small>Next scheduled</small><strong>{overview.nextIngestionAt ? timeAgoFuture(overview.nextIngestionAt) : "Not scheduled"}</strong><p>{formatDate(overview.nextIngestionAt, true)}</p></div></div></div>{overview.latestRun && <div className="latest-run"><div><span>Requests</span><strong>{overview.latestRun.attemptedRequests}/{overview.latestRun.totalQueries}</strong></div><div><span>New</span><strong>{overview.latestRun.newJobs}</strong></div><div><span>Errors</span><strong>{overview.latestRun.failedQueries + overview.latestRun.extractionFailures}</strong></div></div>}</SurfaceCard><ActivityGrid activity={overview.activity} /></div></section></div>;
}

function Metric({ label, value, icon: Icon, tone, note }: { label: string; value: number; icon: typeof BriefcaseBusiness; tone: string; note: string }) {
  return <SurfaceCard className={`metric-card metric-card--${tone}`}><div className="metric-card__top"><TechLabel>{label}</TechLabel><span><Icon size={18} /></span></div><strong className="metric-card__value">{compactNumber(value)}</strong><small>{note}</small></SurfaceCard>;
}

function ActivityGrid({ activity }: { activity: Overview["activity"] }) {
  const byDate = new Map(activity.map((day) => [day.date.slice(0, 10), day]));
  const days = Array.from({ length: 35 }, (_, index) => {
    const date = new Date(); date.setDate(date.getDate() - (34 - index));
    const key = date.toISOString().slice(0, 10);
    return byDate.get(key) ?? { date: key, status: "empty", newJobs: 0 };
  });
  return <SurfaceCard className="activity-panel"><div className="panel-heading"><div><TechLabel>35-day activity</TechLabel><h2>Discovery pulse</h2></div><Clock3 size={18} /></div><div className="activity-grid" role="img" aria-label="Ingestion activity over the last 35 days">{days.map((day) => <span key={day.date} className={`activity-cell activity-cell--${day.status}`} title={`${formatDate(day.date)} · ${day.status}${day.newJobs ? ` · ${day.newJobs} new` : ""}`} />)}</div><div className="activity-legend"><span><i className="activity-cell activity-cell--empty" />No run</span><span><i className="activity-cell activity-cell--completed" />Completed</span><span><i className="activity-cell activity-cell--partial" />Partial</span></div></SurfaceCard>;
}

function timeAgoFuture(value: string) {
  const delta = new Date(value).getTime() - Date.now();
  if (delta <= 0) return "Due now";
  const hours = Math.ceil(delta / 3_600_000);
  return hours < 24 ? `In ${hours} hour${hours === 1 ? "" : "s"}` : `In ${Math.ceil(hours / 24)} days`;
}
