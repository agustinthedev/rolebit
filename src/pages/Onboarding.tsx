import { ArrowLeft, ArrowRight, Check, Database, KeyRound, Play, Search, ShieldCheck, Sparkles } from "lucide-react";
import { useEffect, useState } from "react";
import { api } from "../lib/api";
import { currency } from "../lib/format";
import type { AppSettings, Bootstrap, QueryPreview, SaveSettingsInput, SearchConfiguration } from "../types";
import { BrandMark } from "../components/BrandMark";
import { SearchListEditor } from "../components/SearchListEditor";
import { Button, Field, InlineNotice, Input, SurfaceCard, Switch, TechLabel, cn } from "../components/ui";

const steps = ["Welcome", "Bright Data", "Search lists", "Scheduling", "Review", "Complete"];
const draftKey = "rolebit:onboarding-draft";

const defaults: SearchConfiguration = {
  sites: ["jobs.ashbyhq.com", "jobs.lever.co", "job-boards.greenhouse.io"].map((value) => ({ value, enabled: true })),
  locations: ["Uruguay", "LATAM", "Remote", "Worldwide"].map((value) => ({ value, enabled: true })),
  roles: ["QA", "SDET", "Software Engineer", "DevOps", "Product"].map((value) => ({ value, enabled: true })),
  pagesPerQuery: 1,
};

function editableSettings(settings: AppSettings): SaveSettingsInput {
  const { provider, serpZone, googleGl, googleHl, requestConcurrency, requestTimeoutSeconds, retryCount, pagesPerQuery, estimatedCostPerRequest, requestBudget, scheduleEnabled, scheduleIntervalHours, runOnStartup, startWithWindows, closeToTray } = settings;
  return { provider, serpZone, googleGl, googleHl, requestConcurrency, requestTimeoutSeconds, retryCount, pagesPerQuery, estimatedCostPerRequest, requestBudget, scheduleEnabled, scheduleIntervalHours, runOnStartup, startWithWindows, closeToTray };
}

export function Onboarding({ initialSettings, initialConfiguration, onComplete }: { initialSettings: AppSettings; initialConfiguration: SearchConfiguration; onComplete: (bootstrap: Bootstrap) => void }) {
  const draft = readDraft();
  const [step, setStep] = useState(draft?.step ?? 0);
  const [settings, setSettings] = useState<SaveSettingsInput>(draft?.settings ?? editableSettings(initialSettings));
  const [configuration, setConfiguration] = useState(draft?.configuration ?? initialConfiguration);
  const [apiKey, setApiKey] = useState("");
  const [preview, setPreview] = useState<QueryPreview>();
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string>();
  const [error, setError] = useState<string>();

  useEffect(() => {
    const timer = window.setTimeout(() => api.previewQueries({ configuration: { ...configuration, pagesPerQuery: settings.pagesPerQuery }, scheduleIntervalHours: settings.scheduleIntervalHours, estimatedCostPerRequest: settings.estimatedCostPerRequest }).then(setPreview).catch(() => undefined), 120);
    return () => window.clearTimeout(timer);
  }, [configuration, settings.pagesPerQuery, settings.scheduleIntervalHours, settings.estimatedCostPerRequest]);
  useEffect(() => {
    window.localStorage.setItem(draftKey, JSON.stringify({ step, settings, configuration }));
  }, [step, settings, configuration]);

  const testConnection = async () => {
    setBusy(true); setError(undefined); setMessage(undefined);
    try { setMessage(await api.testConnection(apiKey || undefined, settings.serpZone)); }
    catch (cause) { setError(String(cause)); }
    finally { setBusy(false); }
  };
  const complete = async () => {
    setBusy(true); setError(undefined);
    try {
      const result = await api.completeOnboarding({ settings: { ...settings, apiKey: apiKey || undefined }, configuration: { ...configuration, pagesPerQuery: settings.pagesPerQuery } });
      window.localStorage.removeItem(draftKey);
      onComplete(result);
    } catch (cause) { setError(String(cause)); setBusy(false); }
  };
  const next = () => {
    if (step === 1 && settings.provider === "bright_data" && (!settings.serpZone.trim() || (!apiKey && !initialSettings.hasApiKey))) {
      setError("Enter both your Bright Data API key and SERP zone, or choose local mock mode."); return;
    }
    if (step === 2 && (preview?.queries.length ?? 0) === 0) { setError("Enable at least one valid site, location, and role combination."); return; }
    setError(undefined); setStep((value) => Math.min(value + 1, 5));
  };

  return <div className="onboarding"><div className="onboarding__aurora" /><header className="onboarding__topbar"><BrandMark /><span>Private by design · Stored on this computer</span></header><div className="onboarding__layout"><aside className="onboarding__steps">{steps.map((label, index) => <div className={cn("onboarding-step", index === step && "onboarding-step--active", index < step && "onboarding-step--done")} key={label}><span>{index < step ? <Check size={14} /> : index + 1}</span><div><strong>{label}</strong><small>{index === step ? "Current step" : index < step ? "Complete" : "Up next"}</small></div></div>)}</aside><main className="onboarding__content"><SurfaceCard className="onboarding__card">{step === 0 && <Welcome />}{step === 1 && <ProviderStep settings={settings} setSettings={setSettings} apiKey={apiKey} setApiKey={setApiKey} hasApiKey={initialSettings.hasApiKey} busy={busy} onTest={testConnection} message={message} />}{step === 2 && <SearchStep configuration={configuration} setConfiguration={setConfiguration} />}{step === 3 && <ScheduleStep settings={settings} setSettings={setSettings} />}{step === 4 && <ReviewStep preview={preview} settings={settings} />}{step === 5 && <CompleteStep preview={preview} />}{error && <InlineNotice tone="error" title="Needs attention">{error}</InlineNotice>}<footer className="onboarding__actions"><Button variant="ghost" icon={ArrowLeft} disabled={step === 0 || busy} onClick={() => setStep((value) => value - 1)}>Back</Button>{step < 5 ? <Button variant="primary" icon={ArrowRight} onClick={next}>Continue</Button> : <Button variant="primary" icon={Play} disabled={busy} onClick={complete}>{busy ? "Saving…" : "Open Rolebit"}</Button>}</footer></SurfaceCard></main></div></div>;
}

function readDraft(): { step: number; settings: SaveSettingsInput; configuration: SearchConfiguration } | undefined {
  try {
    const raw = window.localStorage.getItem(draftKey);
    if (!raw) return undefined;
    const parsed = JSON.parse(raw) as { step?: number; settings?: SaveSettingsInput; configuration?: SearchConfiguration };
    if (!parsed.settings || !parsed.configuration) return undefined;
    return { step: Math.max(0, Math.min(5, parsed.step ?? 0)), settings: parsed.settings, configuration: parsed.configuration };
  } catch {
    return undefined;
  }
}

function Welcome() {
  return <div className="onboarding-panel onboarding-panel--welcome"><TechLabel>Welcome to Rolebit</TechLabel><h1>Your opportunity radar,<br />calmly organized.</h1><p>Rolebit searches the web using strategies you control, verifies job postings, removes duplicates, and keeps everything in a private local library.</p><div className="welcome-grid"><div><span><Database size={19} /></span><strong>Local-first</strong><p>SQLite storage stays on this device and remains browsable offline.</p></div><div><span><ShieldCheck size={19} /></span><strong>You stay in control</strong><p>No paid search runs happen until you explicitly start one.</p></div><div><span><Sparkles size={19} /></span><strong>Built for focus</strong><p>One clean workspace for discovery, review, and saved roles.</p></div></div><InlineNotice tone="warning" title="A quick cost note">Bright Data usage may incur charges. Rolebit previews request volume before every strategy runs.</InlineNotice></div>;
}

function ProviderStep({ settings, setSettings, apiKey, setApiKey, hasApiKey, busy, onTest, message }: { settings: SaveSettingsInput; setSettings: (value: SaveSettingsInput) => void; apiKey: string; setApiKey: (value: string) => void; hasApiKey: boolean; busy: boolean; onTest: () => void; message?: string }) {
  return <div className="onboarding-panel"><TechLabel>Step 2 · Search provider</TechLabel><h2>Choose how you want to begin.</h2><p className="lead">Mock mode is a safe local sandbox. Connect Bright Data when you are ready to discover real postings.</p><div className="provider-choice"><button className={cn("provider-card", settings.provider === "mock" && "provider-card--active")} onClick={() => setSettings({ ...settings, provider: "mock" })}><span><Sparkles size={21} /></span><strong>Local mock provider</strong><p>Exercise every workflow with realistic sample results and no network cost.</p><small>Recommended for first launch</small></button><button className={cn("provider-card", settings.provider === "bright_data" && "provider-card--active")} onClick={() => setSettings({ ...settings, provider: "bright_data" })}><span><Search size={21} /></span><strong>Bright Data SERP</strong><p>Run live Google discovery through your own API credentials.</p><small>Paid provider</small></button></div>{settings.provider === "bright_data" && <div className="form-grid"><Field label="API key" hint={hasApiKey ? "A credential is already stored. Leave blank to keep it." : "Stored with Windows Credential Manager, never SQLite."}><Input type="password" value={apiKey} onChange={(event) => setApiKey(event.target.value)} placeholder={hasApiKey ? "••••••••••••••••" : "Enter API key"} /></Field><Field label="SERP zone" hint="Your Bright Data SERP API zone identifier."><Input value={settings.serpZone} onChange={(event) => setSettings({ ...settings, serpZone: event.target.value })} placeholder="serp_api1" /></Field><div className="form-grid__wide"><InlineNotice tone="warning" title="Explicit test only">Testing the connection can consume one paid SERP request.</InlineNotice><Button icon={KeyRound} disabled={busy} onClick={onTest}>{busy ? "Testing…" : "Test connection"}</Button>{message && <InlineNotice tone="success" title="Connection result">{message}</InlineNotice>}</div></div>}</div>;
}

function SearchStep({ configuration, setConfiguration }: { configuration: SearchConfiguration; setConfiguration: (value: SearchConfiguration) => void }) {
  return <div className="onboarding-panel onboarding-panel--wide"><TechLabel>Step 3 · Search strategy</TechLabel><h2>Build your discovery matrix.</h2><p className="lead">Rolebit combines every enabled site, location, and role. Empty enabled location or role values are supported intentionally.</p><div className="onboarding-lists"><SearchListEditor compact kind="site" title="Sites" description="Job-host domains" items={configuration.sites} onChange={(sites) => setConfiguration({ ...configuration, sites })} onReset={() => setConfiguration({ ...configuration, sites: defaults.sites })} /><SearchListEditor compact kind="location" title="Locations" description="Markets or work modes" items={configuration.locations} onChange={(locations) => setConfiguration({ ...configuration, locations })} onReset={() => setConfiguration({ ...configuration, locations: defaults.locations })} /><SearchListEditor compact kind="role" title="Roles / Terms" description="Titles and keywords" items={configuration.roles} onChange={(roles) => setConfiguration({ ...configuration, roles })} onReset={() => setConfiguration({ ...configuration, roles: defaults.roles })} /></div></div>;
}

function ScheduleStep({ settings, setSettings }: { settings: SaveSettingsInput; setSettings: (value: SaveSettingsInput) => void }) {
  return <div className="onboarding-panel"><TechLabel>Step 4 · Scheduling</TechLabel><h2>Set a quiet, predictable rhythm.</h2><p className="lead">Scheduling runs only while Rolebit is active. Missed time produces at most one catch-up run—never a paid backlog.</p><div className="settings-stack"><Switch checked={settings.scheduleEnabled} onCheckedChange={(scheduleEnabled) => setSettings({ ...settings, scheduleEnabled })} label="Enable scheduled ingestion" description="Run the current search strategy on a recurring interval." /><Field label="Interval"><select className="select" value={settings.scheduleIntervalHours} onChange={(event) => setSettings({ ...settings, scheduleIntervalHours: Number(event.target.value) })} disabled={!settings.scheduleEnabled}><option value={6}>Every 6 hours</option><option value={12}>Every 12 hours</option><option value={24}>Every day</option><option value={168}>Every week</option></select></Field><Switch checked={settings.runOnStartup} onCheckedChange={(runOnStartup) => setSettings({ ...settings, runOnStartup })} label="Run on startup" description="Start one ingestion after Rolebit launches." /><Switch checked={settings.startWithWindows} onCheckedChange={(startWithWindows) => setSettings({ ...settings, startWithWindows })} label="Start with Windows" description="Launch Rolebit quietly when you sign in." /><Switch checked={settings.closeToTray} onCheckedChange={(closeToTray) => setSettings({ ...settings, closeToTray })} label="Keep running in the system tray" description="Closing the window keeps scheduling active." /></div></div>;
}

function ReviewStep({ preview, settings }: { preview?: QueryPreview; settings: SaveSettingsInput }) {
  return <div className="onboarding-panel"><TechLabel>Step 5 · Review</TechLabel><h2>Everything looks intentional.</h2><p className="lead">Review volume before saving. Completing setup does not start ingestion.</p><div className="review-metrics"><div><span>{preview?.enabledSites ?? 0}</span><small>Enabled sites</small></div><div><span>{preview?.enabledLocations ?? 0}</span><small>Locations</small></div><div><span>{preview?.enabledRoles ?? 0}</span><small>Roles</small></div><div className="review-metrics__accent"><span>{preview?.queries.length ?? 0}</span><small>Search combinations</small></div></div><div className="review-summary"><div><span>Requests per execution</span><strong>{preview?.totalRequests ?? 0}</strong></div><div><span>Estimated monthly requests</span><strong>{settings.scheduleEnabled ? preview?.estimatedMonthlyRequests ?? 0 : "Scheduling off"}</strong></div><div><span>Estimated execution cost</span><strong>{currency(preview?.estimatedCost)}</strong></div><div><span>Provider</span><strong>{settings.provider === "mock" ? "Local mock" : "Bright Data SERP"}</strong></div></div><InlineNotice title="Nothing runs automatically now">You will land on the dashboard with a clear Run Ingestion action.</InlineNotice></div>;
}

function CompleteStep({ preview }: { preview?: QueryPreview }) {
  return <div className="onboarding-panel onboarding-panel--complete"><span className="complete-orbit"><Check size={30} /></span><TechLabel>Step 6 · Ready</TechLabel><h2>Your local opportunity library is ready.</h2><p>Rolebit will begin with {preview?.queries.length ?? 0} configured searches. Open the dashboard, inspect the exact queries, and start ingestion whenever you choose.</p><div className="complete-detail"><ShieldCheck size={19} /><span><strong>No search has run yet.</strong> Your configuration is saved locally and remains editable.</span></div></div>;
}
