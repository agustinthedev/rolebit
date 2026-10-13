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

export function plainText(value?: string): string {
  if (!value) return "";
  let current = value;
  for (let pass = 0; pass < 3; pass += 1) {
    const parsed = new DOMParser().parseFromString(current, "text/html");
    const next = normalizeFormattedText(renderNode(parsed.body));
    if (next === current || !looksLikeMarkup(next)) return next;
    current = next;
  }
  return current;
}

const blockTags = new Set(["article", "blockquote", "br", "div", "h1", "h2", "h3", "h4", "h5", "h6", "header", "li", "ol", "p", "section", "ul"]);

function renderNode(node: Node, insideListItem = false): string {
  if (node.nodeType === Node.TEXT_NODE) return node.textContent ?? "";
  if (node.nodeType !== Node.ELEMENT_NODE) return "";
  const element = node as Element;
  const tag = element.tagName.toLowerCase();
  const isListItem = tag === "li";
  const isBlock = blockTags.has(tag);
  let output = "";
  if (isListItem) output += "\n• ";
  else if (isBlock && !insideListItem) output += "\n";
  for (const child of Array.from(element.childNodes)) output += renderNode(child, insideListItem || isListItem);
  if (!isListItem && isBlock && !insideListItem) output += "\n";
  return output;
}

function normalizeFormattedText(value: string): string {
  const lines: string[] = [];
  let blankPending = false;
  for (const line of value.split(/\r?\n/).map((item) => item.trim())) {
    if (!line) {
      if (lines.length) blankPending = true;
      continue;
    }
    if (blankPending) {
      lines.push("");
      blankPending = false;
    }
    lines.push(line.replace(/\s+/g, " "));
  }
  return lines.join("\n");
}

function looksLikeMarkup(value: string): boolean {
  return /<\/?[a-z][^>]*>/i.test(value) || /&lt;\/?[a-z][^>]*&gt;/i.test(value);
}
