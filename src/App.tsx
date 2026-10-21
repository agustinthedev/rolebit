import { AlertTriangle, LoaderCircle, X } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import "./App.css";
import { AppShell } from "./components/AppShell";
import { BrandMark } from "./components/BrandMark";
import { IconButton } from "./components/ui";
import { api } from "./lib/api";
import type { Bootstrap, IngestionProgress, PageId } from "./types";
import { HistoryPage } from "./pages/HistoryPage";
import { Onboarding } from "./pages/Onboarding";
import { OpportunitiesPage } from "./pages/OpportunitiesPage";
import { OverviewPage } from "./pages/OverviewPage";
import { SettingsPage } from "./pages/SettingsPage";
import { ProfilePage } from "./pages/ProfilePage";

function App() {
  const [bootstrap, setBootstrap] = useState<Bootstrap>();
  const [page, setPage] = useState<PageId>("overview");
  const [progress, setProgress] = useState<IngestionProgress>();
  const [refreshKey, setRefreshKey] = useState(0);
  const [selectedJobId, setSelectedJobId] = useState<number>();
  const [loading, setLoading] = useState(true);
  const [starting, setStarting] = useState(false);
  const [error, setError] = useState<string>();

  const refresh = useCallback(async () => {
    try { setBootstrap(await api.bootstrap()); setError(undefined); }
    catch (cause) { setError(String(cause)); }
    finally { setLoading(false); }
  }, []);

  useEffect(() => { void refresh(); }, [refresh]);
  useEffect(() => {
    let disposed = false;
    let unlisten: () => void = () => {};
    api.onProgress((next) => {
      if (disposed) return;
      setProgress(next);
      if (next.status !== "running") {
        setStarting(false); setRefreshKey((value) => value + 1); void refresh();
      }
    }).then((cleanup) => { unlisten = cleanup; });
    return () => { disposed = true; unlisten(); };
  }, [refresh]);

  const startIngestion = async () => {
    setStarting(true); setError(undefined);
    try { await api.startIngestion(); }
    catch (cause) { setError(String(cause)); setStarting(false); }
  };
  const cancel = async () => { try { await api.cancelIngestion(); } catch (cause) { setError(String(cause)); } };
  const showJob = (id: number) => { setSelectedJobId(id); setPage("opportunities"); };

  if (loading) return <div className="app-loading"><BrandMark /><LoaderCircle className="spin" size={24} /><span>Opening your local library…</span></div>;
  if (!bootstrap) return <div className="app-loading app-loading--error"><AlertTriangle size={28} /><h1>Rolebit could not open</h1><p>{error ?? "The local application state is unavailable."}</p><button onClick={refresh}>Try again</button></div>;
  if (!bootstrap.settings.onboardingComplete) return <Onboarding initialSettings={bootstrap.settings} initialConfiguration={bootstrap.configuration} onComplete={setBootstrap} />;

  return <AppShell page={page} onPageChange={setPage}>{error && <div className="global-toast"><AlertTriangle size={17} /><span>{error}</span><IconButton aria-label="Dismiss error" onClick={() => setError(undefined)}><X size={16} /></IconButton></div>}{page === "overview" && <OverviewPage overview={bootstrap.overview} progress={progress} onStart={startIngestion} onCancel={cancel} onNavigate={setPage} onSelectJob={showJob} busy={starting} />}{page === "opportunities" && <OpportunitiesPage refreshKey={refreshKey} initialJobId={selectedJobId} onClearInitial={() => setSelectedJobId(undefined)} />}{page === "history" && <HistoryPage refreshKey={refreshKey} />}{page === "profile" && <ProfilePage />}{page === "settings" && <SettingsPage initial={bootstrap.settings} initialConfiguration={bootstrap.configuration} onSaved={(settings) => setBootstrap((current) => current && { ...current, settings })} onConfigurationSaved={(configuration) => setBootstrap((current) => current && { ...current, configuration })} />}</AppShell>;
}

export default App;
