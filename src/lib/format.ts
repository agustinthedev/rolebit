import { format, formatDistanceToNowStrict, isValid, parseISO } from "date-fns";

export function formatDate(value?: string, includeTime = false): string {
  if (!value) return "Not yet";
  const date = parseISO(value);
  if (!isValid(date)) return value;
  return format(date, includeTime ? "MMM d, yyyy · HH:mm" : "MMM d, yyyy");
}

export function timeAgo(value?: string): string {
  if (!value) return "Never";
  const date = parseISO(value);
  if (!isValid(date)) return value;
  return `${formatDistanceToNowStrict(date)} ago`;
}

export function compactNumber(value: number): string {
  return new Intl.NumberFormat(undefined, { notation: value > 999 ? "compact" : "standard" }).format(value);
}

export function currency(value?: number): string {
  if (value === undefined) return "Not configured";
  return new Intl.NumberFormat(undefined, { style: "currency", currency: "USD", maximumFractionDigits: 4 }).format(value);
}

export function fileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 ** 2) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1024 ** 2).toFixed(1)} MB`;
}
