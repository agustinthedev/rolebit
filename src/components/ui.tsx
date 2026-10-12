import * as DialogPrimitive from "@radix-ui/react-dialog";
import * as SwitchPrimitive from "@radix-ui/react-switch";
import { X, type LucideIcon } from "lucide-react";
import type { ButtonHTMLAttributes, HTMLAttributes, InputHTMLAttributes, ReactNode } from "react";
import { twMerge } from "tailwind-merge";
import clsx, { type ClassValue } from "clsx";

export function cn(...values: ClassValue[]) {
  return twMerge(clsx(values));
}

type ButtonProps = ButtonHTMLAttributes<HTMLButtonElement> & { variant?: "primary" | "secondary" | "ghost" | "danger"; icon?: LucideIcon };

export function Button({ className, variant = "secondary", icon: Icon, children, ...props }: ButtonProps) {
  return <button className={cn("button", `button--${variant}`, className)} {...props}>{Icon && <Icon size={16} strokeWidth={2} />}{children}</button>;
}

export function IconButton({ className, children, ...props }: ButtonHTMLAttributes<HTMLButtonElement>) {
  return <button className={cn("icon-button", className)} {...props}>{children}</button>;
}

export function SurfaceCard({ className, children, ...props }: HTMLAttributes<HTMLDivElement>) {
  return <div className={cn("surface-card", className)} {...props}>{children}</div>;
}

export function TechLabel({ children, className }: { children: ReactNode; className?: string }) {
  return <span className={cn("tech-label", className)}>{children}</span>;
}

export function StatusBadge({ value, children }: { value: string; children?: ReactNode }) {
  return <span className={cn("status-badge", `status-badge--${value.replaceAll("_", "-")}`)}><span className="status-badge__dot" />{children ?? value.replaceAll("_", " ")}</span>;
}

export function PageHeader({ eyebrow, title, description, actions }: { eyebrow?: string; title: string; description: string; actions?: ReactNode }) {
  return <header className="page-header"><div>{eyebrow && <TechLabel>{eyebrow}</TechLabel>}<h1>{title}</h1><p>{description}</p></div>{actions && <div className="page-header__actions">{actions}</div>}</header>;
}

export function Field({ label, hint, error, children, className }: { label: string; hint?: string; error?: string; children: ReactNode; className?: string }) {
  return <label className={cn("field", className)}><span className="field__label">{label}</span>{children}{hint && <span className="field__hint">{hint}</span>}{error && <span className="field__error">{error}</span>}</label>;
}

export function Input({ className, ...props }: InputHTMLAttributes<HTMLInputElement>) {
  return <input className={cn("input", className)} {...props} />;
}

export function Select({ className, children, ...props }: React.SelectHTMLAttributes<HTMLSelectElement>) {
  return <select className={cn("select", className)} {...props}>{children}</select>;
}

export function Switch({ checked, onCheckedChange, label, description, disabled }: { checked: boolean; onCheckedChange: (checked: boolean) => void; label: string; description?: string; disabled?: boolean }) {
  return <div className="switch-field"><div><div className="switch-field__label">{label}</div>{description && <div className="switch-field__description">{description}</div>}</div><SwitchPrimitive.Root className="switch" checked={checked} onCheckedChange={onCheckedChange} disabled={disabled} aria-label={label}><SwitchPrimitive.Thumb className="switch__thumb" /></SwitchPrimitive.Root></div>;
}

export function EmptyState({ icon: Icon, title, description, action }: { icon: LucideIcon; title: string; description: string; action?: ReactNode }) {
  return <div className="empty-state"><span className="empty-state__icon"><Icon size={24} /></span><h3>{title}</h3><p>{description}</p>{action}</div>;
}

export function Modal({ open, onOpenChange, title, description, children, className }: { open: boolean; onOpenChange: (open: boolean) => void; title: string; description?: string; children: ReactNode; className?: string }) {
  return <DialogPrimitive.Root open={open} onOpenChange={onOpenChange}><DialogPrimitive.Portal><DialogPrimitive.Overlay className="dialog-overlay" /><DialogPrimitive.Content className={cn("dialog-content", className)}><div className="dialog-header"><div><DialogPrimitive.Title>{title}</DialogPrimitive.Title>{description && <DialogPrimitive.Description>{description}</DialogPrimitive.Description>}</div><DialogPrimitive.Close asChild><IconButton aria-label="Close"><X size={18} /></IconButton></DialogPrimitive.Close></div>{children}</DialogPrimitive.Content></DialogPrimitive.Portal></DialogPrimitive.Root>;
}

export function Skeleton({ className }: { className?: string }) {
  return <span className={cn("skeleton", className)} />;
}

export function InlineNotice({ tone = "neutral", title, children }: { tone?: "neutral" | "success" | "warning" | "error"; title: string; children: ReactNode }) {
  return <div className={cn("inline-notice", `inline-notice--${tone}`)}><strong>{title}</strong><span>{children}</span></div>;
}
