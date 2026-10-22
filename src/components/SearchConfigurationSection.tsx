import { Clipboard, Eye, Sigma, Sparkles } from "lucide-react";
import { useEffect, useState } from "react";
import { SearchListEditor } from "./SearchListEditor";
import { Button, InlineNotice, Modal, SurfaceCard, TechLabel } from "./ui";
import { api } from "../lib/api";
import { currency } from "../lib/format";
import type { QueryPreview, SaveSettingsInput, SearchConfiguration } from "../types";

const defaultItems = {
  sites: ["jobs.ashbyhq.com", "jobs.lever.co", "job-boards.greenhouse.io"].map((value) => ({ value, enabled: true })),
  locations: ["Uruguay", "LATAM", "Remote", "Worldwide"].map((value) => ({ value, enabled: true })),
};

type RequestSettings = Pick<SaveSettingsInput, "pagesPerQuery" | "scheduleEnabled" | "scheduleIntervalHours" | "estimatedCostPerRequest">;

export function SearchConfigurationSection({ configuration, savedConfiguration, settings, onChange }: { configuration: SearchConfiguration; savedConfiguration: SearchConfiguration; settings: RequestSettings; onChange: (configuration: SearchConfiguration) => void }) {
  const [preview, setPreview] = useState<QueryPreview>();
  const [showPreview, setShowPreview] = useState(false);
  const [notice, setNotice] = useState<string>();
  const [error, setError] = useState<string>();
  const isDirty = JSON.stringify(configuration) !== JSON.stringify(savedConfiguration);

  useEffect(() => {
    const timer = window.setTimeout(() => api.previewQueries({ configuration: { ...configuration, pagesPerQuery: settings.pagesPerQuery }, scheduleIntervalHours: settings.scheduleIntervalHours, estimatedCostPerRequest: settings.estimatedCostPerRequest }).then(setPreview).catch((cause) => setError(String(cause))), 160);
    return () => window.clearTimeout(timer);
  }, [configuration, settings.pagesPerQuery, settings.scheduleIntervalHours, settings.estimatedCostPerRequest]);

  const copy = async () => { await navigator.clipboard.writeText(preview?.queries.join("\n") ?? ""); setNotice("Queries copied to the clipboard."); };

  return <SurfaceCard className="settings-section search-configuration-section">
    <div className="search-configuration__heading"><div className="settings-section__heading"><span><Sigma size={19} /></span><div><TechLabel>Search strategy</TechLabel><h2>Search configuration</h2><p>Control which sites and locations feed discovery.</p></div></div><div className="search-configuration__actions"><Button icon={Eye} onClick={() => setShowPreview(true)}>Preview queries</Button></div></div>
    <section className="configuration-layout"><div className="configuration-lists"><SearchListEditor kind="site" title="Sites" description="Domains or URL patterns where job postings live." items={configuration.sites} onChange={(sites) => onChange({ ...configuration, sites })} onReset={() => onChange({ ...configuration, sites: defaultItems.sites })} /><SearchListEditor kind="location" title="Locations" description="Geographies or work arrangements." items={configuration.locations} onChange={(locations) => onChange({ ...configuration, locations })} onReset={() => onChange({ ...configuration, locations: defaultItems.locations })} /></div><aside className="configuration-side"><SurfaceCard className="query-metric"><span className="query-metric__icon"><Sigma size={22} /></span><TechLabel>Total combinations</TechLabel><strong>{preview?.queries.length ?? 0}</strong><p>{preview?.enabledSites ?? 0} sites × {preview?.enabledLocations ?? 0} locations</p></SurfaceCard><SurfaceCard className="execution-estimate"><TechLabel>Execution estimate</TechLabel><h3>Know the volume first.</h3><div><span>Planned requests</span><strong>{preview?.totalRequests ?? 0}</strong></div><div><span>Monthly requests</span><strong>{settings.scheduleEnabled ? preview?.estimatedMonthlyRequests ?? 0 : "Schedule off"}</strong></div><div><span>Estimated cost</span><strong>{currency(preview?.estimatedCost)}</strong></div><small>Cost uses your own per-request estimate. It is not confirmed billing.</small>{isDirty && <InlineNotice tone="warning" title="Unsaved changes">Use the Save settings button at the top of the page to apply this search strategy.</InlineNotice>}</SurfaceCard>{notice && <InlineNotice tone="success" title="Copied">{notice}</InlineNotice>}{error && <InlineNotice tone="error" title="Could not load preview">{error}</InlineNotice>}</aside></section>
    <Modal open={showPreview} onOpenChange={setShowPreview} title="Generated query preview" description={`${preview?.queries.length ?? 0} deterministic Google searches · ${preview?.totalRequests ?? 0} planned requests`} className="query-dialog"><div className="query-dialog__toolbar"><InlineNotice title="Exact execution plan">Rolebit preserves this list as written and records each query as discovery attribution.</InlineNotice><Button icon={Clipboard} onClick={copy}>Copy queries</Button></div>{preview?.queries.length ? <ol className="query-list">{preview.queries.map((query, index) => <li key={query}><span>{String(index + 1).padStart(2, "0")}</span><code>{query}</code></li>)}</ol> : <div className="query-empty"><Sparkles size={24} /><h3>No valid combinations</h3><p>Enable at least one site and location.</p></div>}</Modal>
  </SurfaceCard>;
}
