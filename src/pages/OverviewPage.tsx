import { ArrowRight, BriefcaseBusiness, CalendarClock, CircleCheck, RefreshCw, Sparkles, Target, TrendingUp, UserRound, X } from "lucide-react";
import { compactNumber, formatDate, timeAgo } from "../lib/format";
import type { IngestionProgress, JobSummary, Overview, PageId } from "../types";
import { Button, EmptyState, MatchBadge, PageHeader, StatusBadge, SurfaceCard, TechLabel } from "../components/ui";

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
  const roles = overview.recentJobs;
  const featured = roles.find((job) => job.matchScore != null);
  const hasScoringErrors = roles.some((job) => job.matchError);
  const shortlist = roles.filter((job) => job.id !== featured?.id).slice(0, featured ? 4 : 5);

  return <div className="page overview-page">
    <PageHeader eyebrow="AI-powered career matching" title="Your next role, found." description="A personal shortlist ranked by how each opportunity fits your experience, skills, seniority, and eligibility." actions={running ? <Button variant="secondary" icon={X} onClick={onCancel}>Cancel run</Button> : <Button variant="primary" icon={RefreshCw} disabled={busy} onClick={onStart}>{busy ? "Starting…" : "Find and match roles"}</Button>} />
    <section className="overview-layout">
      <div className="overview-main">
        <SurfaceCard className="match-dashboard-panel">
          <div className="panel-heading match-panel-heading"><div><TechLabel>Profile fit</TechLabel><h2>Recommended for you</h2><p>Ranked by match quality across every active opportunity, whether it is new, saved, or already reviewed.</p></div><Button variant="ghost" icon={ArrowRight} onClick={() => onNavigate("opportunities")}>All opportunities</Button></div>
          {!featured && roles.length > 0 && <div className="match-not-ready"><span>{hasScoringErrors ? <Target size={19} /> : <UserRound size={19} />}</span><div><strong>{hasScoringErrors ? "Some profile assessments need a retry" : "Profile matches are not ready yet"}</strong><p>{hasScoringErrors ? "Run another search to retry Decisions API scoring for these roles." : "Add professional experience or skills and configure a Decisions API key, then run a search."}</p></div><Button variant="secondary" onClick={() => hasScoringErrors ? onStart() : onNavigate("profile")}>{hasScoringErrors ? "Retry assessments" : "Complete profile"}</Button></div>}
          {featured && <FeaturedMatch job={featured} onOpen={() => onSelectJob(featured.id)} />}
          {shortlist.length > 0 && <div className="match-shortlist">{shortlist.map((job, index) => <MatchRow key={job.id} job={job} rank={index + (featured ? 2 : 1)} onOpen={() => onSelectJob(job.id)} />)}</div>}
          {!roles.length && <EmptyState icon={BriefcaseBusiness} title="Your opportunity library is waiting" description="Configure your search and Decisions API, then run an ingestion. Rolebit will classify relevant postings and assess them against your profile." action={<Button variant="primary" icon={RefreshCw} onClick={onStart}>Run first search</Button>} />}
        </SurfaceCard>
        {running && progress && <SurfaceCard className="ingestion-live"><div className="ingestion-live__header"><div><TechLabel>Live search</TechLabel><h3>Finding roles that fit</h3></div><span>{progress.completedQueries} / {progress.totalQueries}</span></div><div className="progress-track"><span style={{ width: `${Math.max(2, progress.totalQueries ? progress.completedQueries / progress.totalQueries * 100 : 2)}%` }} /></div><div className="ingestion-live__stats"><span><strong>{progress.urlsDiscovered}</strong> URLs discovered</span><span><strong>{progress.newJobs}</strong> new roles</span><span><strong>{progress.updatedJobs}</strong> updated</span><span><strong>{progress.filteredResults}</strong> filtered</span><span><strong>{progress.errors}</strong> errors</span></div><p>{progress.currentQuery ?? "Preparing queries…"}</p></SurfaceCard>}
      </div>
      <div className="overview-side">
        <SurfaceCard className="status-panel"><div className="panel-heading"><div><TechLabel>Search activity</TechLabel><h2>Your search rhythm</h2></div><StatusBadge value={running ? "running" : overview.ingestionStatus} /></div><div className="overview-counts"><div><strong>{compactNumber(overview.strongMatches)}</strong><span>75%+ profile fits</span></div><div><strong>{compactNumber(overview.newOpportunities)}</strong><span>new to review</span></div><div><strong>{compactNumber(overview.totalOpportunities)}</strong><span>roles in your library</span></div><div><strong>{compactNumber(overview.activeQueries)}</strong><span>active search queries</span></div></div><div className="status-timeline"><div><span><CircleCheck size={17} /></span><div><small>Last search</small><strong>{timeAgo(overview.lastIngestionAt)}</strong><p>{formatDate(overview.lastIngestionAt, true)}</p></div></div><div><span><CalendarClock size={17} /></span><div><small>Next scheduled</small><strong>{overview.nextIngestionAt ? timeAgoFuture(overview.nextIngestionAt) : "Not scheduled"}</strong><p>{formatDate(overview.nextIngestionAt, true)}</p></div></div></div>{overview.latestRun && <div className="latest-run"><div><span>Requests</span><strong>{overview.latestRun.attemptedRequests}/{overview.latestRun.totalQueries}</strong></div><div><span>New</span><strong>{overview.latestRun.newJobs}</strong></div><div><span>Updated</span><strong>{overview.latestRun.updatedJobs}</strong></div><div><span>Errors</span><strong>{overview.latestRun.failedQueries + overview.latestRun.extractionFailures}</strong></div></div>}</SurfaceCard>
        <ActivityGrid activity={overview.activity} />
      </div>
    </section>
  </div>;
}

function FeaturedMatch({ job, onOpen }: { job: JobSummary; onOpen: () => void }) {
  return <button className="featured-match" onClick={onOpen}>
    <div className="featured-match__visual"><div className="match-orbit" style={{ background: `conic-gradient(var(--mint) 0% ${job.matchScore ?? 0}%, rgba(113,101,221,.13) ${job.matchScore ?? 0}% 100%)` }}><div><strong>{job.matchScore}%</strong><small>profile fit</small></div></div><span><Sparkles size={13} /> Best fit</span></div>
    <div className="featured-match__content"><div className="featured-match__topline"><MatchBadge score={job.matchScore} error={job.matchError} /><StatusBadge value={job.trackingStatus} /></div><h3>{job.title}</h3><p className="featured-match__company">{job.company ?? "Company unknown"} · {job.location ?? "Location not specified"}</p><p className="featured-match__reason">{job.matchRationale ?? "Open the assessment to review the profile fit dimensions."}</p><div className="featured-match__footer"><span><Target size={14} /> Assessed against your profile</span><span>{timeAgo(job.firstSeenAt)}</span><ArrowRight size={16} /></div></div>
  </button>;
}

function MatchRow({ job, rank, onOpen }: { job: JobSummary; rank: number; onOpen: () => void }) {
  const score = job.matchScore;
  return <button className={`match-row${score == null ? " match-row--unscored" : ""}`} onClick={onOpen}>
    <span className="match-row__rank">{String(rank).padStart(2, "0")}</span>
    <span className="match-row__identity"><strong>{job.title}</strong><small>{job.company ?? "Company unknown"} · {job.location ?? "Location not specified"}</small><span>{job.matchRationale ?? "No profile assessment yet"}</span></span>
    <span className="match-row__fit"><MatchBadge score={score} error={job.matchError} compact /><i><span style={{ width: `${score ?? 0}%` }} /></i></span>
    <StatusBadge value={job.trackingStatus} />
    <ArrowRight size={16} />
  </button>;
}

function ActivityGrid({ activity }: { activity: Overview["activity"] }) {
  const byDate = new Map(activity.map((day) => [day.date.slice(0, 10), day]));
  const days = Array.from({ length: 35 }, (_, index) => {
    const date = new Date(); date.setDate(date.getDate() - (34 - index));
    const key = date.toISOString().slice(0, 10);
    return byDate.get(key) ?? { date: key, status: "empty", newJobs: 0 };
  });
  return <SurfaceCard className="activity-panel"><div className="panel-heading"><div><TechLabel>35-day activity</TechLabel><h2>Discovery pulse</h2></div><TrendingUp size={18} /></div><div className="activity-grid" role="img" aria-label="Ingestion activity over the last 35 days">{days.map((day) => <span key={day.date} className={`activity-cell activity-cell--${day.status}`} title={`${formatDate(day.date)} · ${day.status}${day.newJobs ? ` · ${day.newJobs} new` : ""}`} />)}</div><div className="activity-legend"><span><i className="activity-cell activity-cell--empty" />No run</span><span><i className="activity-cell activity-cell--completed" />Completed</span><span><i className="activity-cell activity-cell--partial" />Partial</span></div></SurfaceCard>;
}

function timeAgoFuture(value: string) {
  const delta = new Date(value).getTime() - Date.now();
  if (delta <= 0) return "Due now";
  const hours = Math.ceil(delta / 3_600_000);
  return hours < 24 ? `In ${hours} hour${hours === 1 ? "" : "s"}` : `In ${Math.ceil(hours / 24)} days`;
}
