import { Clock3, LayoutDashboard, ListFilter, Settings2, Sparkles, UserRound } from "lucide-react";
import type { ReactNode } from "react";
import type { PageId } from "../types";
import { isDesktop } from "../lib/api";
import { BrandMark } from "./BrandMark";
import { cn } from "./ui";

const items = [
  { id: "overview", label: "Overview", icon: LayoutDashboard },
  { id: "opportunities", label: "All Opportunities", icon: ListFilter },
  { id: "history", label: "Ingestion History", icon: Clock3 },
  { id: "profile", label: "Profile", icon: UserRound },
  { id: "settings", label: "Settings", icon: Settings2 },
] as const;

export function AppShell({ page, onPageChange, children }: { page: PageId; onPageChange: (page: PageId) => void; children: ReactNode }) {
  return <div className="app-atmosphere"><div className="aurora aurora--lavender" /><div className="aurora aurora--peach" /><div className="aurora aurora--mint" /><div className="app-shell"><aside className="sidebar"><BrandMark /><nav className="sidebar__nav" aria-label="Primary navigation">{items.map(({ id, label, icon: Icon }) => <button key={id} className={cn("nav-item", page === id && "nav-item--active")} onClick={() => onPageChange(id)}><Icon size={18} /><span>{label}</span></button>)}</nav><div className="sidebar__footer"><div className="local-card"><span className="local-card__icon"><Sparkles size={16} /></span><div><strong>Local & private</strong><p>Your opportunities stay on this computer.</p></div></div>{!isDesktop() && <span className="preview-badge">Browser preview · demo data</span>}</div></aside><main className="main-content">{children}</main></div></div>;
}
