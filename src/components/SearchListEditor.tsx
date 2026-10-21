import { Globe2, MapPin, Plus, RotateCcw, SearchCheck, Trash2, type LucideIcon } from "lucide-react";
import type { SearchItem } from "../types";
import { Button, IconButton, Input, SurfaceCard, Switch, TechLabel } from "./ui";

const icons: Record<string, LucideIcon> = { site: Globe2, location: MapPin, role: SearchCheck };

export function SearchListEditor({ kind, title, description, items, onChange, onReset, compact = false }: {
  kind: "site" | "location" | "role";
  title: string;
  description: string;
  items: SearchItem[];
  onChange: (items: SearchItem[]) => void;
  onReset?: () => void;
  compact?: boolean;
}) {
  const Icon = icons[kind];
  const update = (index: number, patch: Partial<SearchItem>) => onChange(items.map((item, itemIndex) => itemIndex === index ? { ...item, ...patch } : item));
  const remove = (index: number) => onChange(items.filter((_, itemIndex) => itemIndex !== index));
  const add = () => onChange([...items, { value: "", enabled: true }]);
  return <SurfaceCard className={compact ? "list-editor list-editor--compact" : "list-editor"}><div className="list-editor__header"><div className="list-editor__title"><span className={`list-editor__icon list-editor__icon--${kind}`}><Icon size={18} /></span><div><TechLabel>{kind}</TechLabel><h3>{title}</h3><p>{description}</p></div></div>{onReset && <Button variant="ghost" icon={RotateCcw} onClick={onReset}>Defaults</Button>}</div><div className="list-editor__items">{items.map((item, index) => <div className="list-editor__row" key={item.id ?? `${kind}-${index}`}><Switch checked={item.enabled} onCheckedChange={(enabled) => update(index, { enabled })} label={`${item.enabled ? "Disable" : "Enable"} ${item.value || kind}`} /><Input value={item.value} onChange={(event) => update(index, { value: event.target.value })} placeholder={kind === "site" ? "jobs.example.com/jobs/*" : kind === "location" ? "Remote" : "Software Engineer"} aria-label={`${title} item ${index + 1}`} /><IconButton onClick={() => remove(index)} aria-label={`Remove ${item.value || kind}`}><Trash2 size={16} /></IconButton></div>)}</div><Button variant="ghost" icon={Plus} onClick={add}>Add {kind}</Button></SurfaceCard>;
}
