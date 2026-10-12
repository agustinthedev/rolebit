import { mkdir } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const browser = await chromium.launch({
  executablePath: "C:\\Program Files\\BraveSoftware\\Brave-Browser\\Application\\brave.exe",
  headless: true,
});

const output = new URL("../docs/screenshots/", import.meta.url);
await mkdir(output, { recursive: true });

async function shot(page, name) {
  await page.waitForTimeout(250);
  await page.screenshot({
    path: fileURLToPath(new URL(`${name}.png`, output)),
    animations: "disabled",
  });
}

const context = await browser.newContext({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 1 });
const page = await context.newPage();
await page.goto("http://127.0.0.1:1420/", { waitUntil: "networkidle" });
await page.getByRole("heading", { name: "Good morning." }).waitFor();
await shot(page, "overview");

await page.getByRole("button", { name: "All Opportunities" }).click();
await page.getByRole("heading", { name: "All Opportunities" }).waitFor();
await page.getByRole("cell", { name: /Senior QA Engineer/ }).waitFor();
await shot(page, "opportunities");
await page.getByRole("cell", { name: /Senior QA Engineer/ }).click();
await page.getByRole("dialog").waitFor();
await shot(page, "opportunity-detail");
await page.keyboard.press("Escape");

await page.getByRole("button", { name: "Search Configuration" }).click();
await page.getByRole("heading", { name: "Search Configuration" }).waitFor();
await shot(page, "search-configuration");

await page.getByRole("button", { name: "Ingestion History" }).click();
await page.getByRole("heading", { name: "Ingestion History" }).waitFor();
await shot(page, "ingestion-history");

await page.getByRole("button", { name: "Settings" }).click();
await page.getByRole("heading", { name: "Settings" }).waitFor();
await shot(page, "settings");

const onboarding = await context.newPage();
await onboarding.goto("http://127.0.0.1:1420/?onboarding=1", { waitUntil: "networkidle" });
await onboarding.getByRole("heading", { name: /Your opportunity radar/ }).waitFor();
await shot(onboarding, "onboarding");

await page.setViewportSize({ width: 1280, height: 800 });
await page.getByRole("button", { name: "Overview" }).click();
await page.getByRole("heading", { name: "Good morning." }).waitFor();
await shot(page, "overview-1280");

await page.setViewportSize({ width: 1920, height: 1080 });
await shot(page, "overview-1920");

await browser.close();
