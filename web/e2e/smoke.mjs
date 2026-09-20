// Opt-in browser smoke test for the hosted demo (Playwright).
//
//   cd web
//   npm install
//   npx playwright install chromium
//   npm run test:e2e                       # tests https://card.batuhan4.com
//   E2E_BASE_URL=http://localhost:8788 npm run test:e2e   # or a local deploy
//
// It loads every page at mobile, tablet and desktop widths, flags horizontal
// overflow, console/page errors and failed requests, exercises the reveal
// button on /cards (real edge API call), and writes screenshots to
// e2e/artifacts/. It exits non-zero on any finding.

import { chromium } from "playwright";
import { mkdirSync } from "node:fs";

const BASE = process.env.E2E_BASE_URL ?? "https://card.batuhan4.com";
const OUT = process.env.E2E_OUT ?? new URL("./artifacts/", import.meta.url).pathname;
const PAGES = ["/", "/quick", "/dashboard", "/deposit", "/cards"];
const VIEWPORTS = [
  { name: "mobile-390x844", width: 390, height: 844, isMobile: true, hasTouch: true, deviceScaleFactor: 2 },
  { name: "mobile-360x800", width: 360, height: 800, isMobile: true, hasTouch: true, deviceScaleFactor: 2 },
  { name: "tablet-768x1024", width: 768, height: 1024, isMobile: true, hasTouch: true, deviceScaleFactor: 2 },
  { name: "desktop-1440x900", width: 1440, height: 900, isMobile: false, hasTouch: false, deviceScaleFactor: 1 },
];

mkdirSync(OUT, { recursive: true });

const browser = await chromium.launch();
const failures = [];
let checks = 0;

for (const vp of VIEWPORTS) {
  const context = await browser.newContext({
    viewport: { width: vp.width, height: vp.height },
    isMobile: vp.isMobile,
    hasTouch: vp.hasTouch,
    deviceScaleFactor: vp.deviceScaleFactor,
  });

  for (const path of PAGES) {
    const page = await context.newPage();
    const consoleErrors = [];
    const pageErrors = [];
    const failed = [];
    page.on("console", (msg) => {
      if (msg.type() === "error") consoleErrors.push(msg.text().slice(0, 200));
    });
    page.on("pageerror", (err) => pageErrors.push(String(err).slice(0, 200)));
    page.on("requestfailed", (req) => {
      // Ignore aborted Next.js prefetches.
      if (req.method() !== "HEAD") {
        failed.push(`${req.method()} ${req.url().slice(0, 120)} ${req.failure()?.errorText ?? ""}`);
      }
    });

    let status = 0;
    try {
      const response = await page.goto(`${BASE}${path}`, {
        waitUntil: "networkidle",
        timeout: 45000,
      });
      status = response?.status() ?? 0;
    } catch (error) {
      pageErrors.push(`navigation: ${String(error).slice(0, 200)}`);
    }
    await page.waitForTimeout(1000);

    const overflow = await page.evaluate((expectedWidth) => {
      const doc = document.documentElement;
      const offenders = [];
      const zoomedOut = window.innerWidth !== expectedWidth;
      if (doc.scrollWidth > expectedWidth + 1 || zoomedOut) {
        for (const el of Array.from(document.querySelectorAll("body *"))) {
          const rect = el.getBoundingClientRect();
          if (rect.right > doc.clientWidth + 1 && rect.width > 24) {
            offenders.push(
              `${el.tagName.toLowerCase()}.${String(el.className).split(" ").slice(0, 2).join(".")} right=${Math.round(rect.right)}`
            );
          }
          if (offenders.length >= 5) break;
        }
      }
      return {
        overflow: doc.scrollWidth > expectedWidth + 1,
        zoomedOut,
        offenders,
      };
    }, vp.width);

    const slug = path === "/" ? "home" : path.slice(1).replace(/\//g, "-");
    await page.screenshot({ path: `${OUT}/${slug}-${vp.name}.png`, fullPage: true });

    checks += 1;
    if (status !== 200 || overflow.overflow || overflow.zoomedOut || consoleErrors.length || pageErrors.length || failed.length) {
      failures.push({
        viewport: vp.name,
        path,
        status,
        overflow: overflow.overflow,
        zoomedOut: overflow.zoomedOut,
        offenders: overflow.offenders,
        consoleErrors: consoleErrors.slice(0, 3),
        pageErrors: pageErrors.slice(0, 3),
        failedRequests: failed.slice(0, 3),
      });
    }
    await page.close();
  }
  await context.close();
}

// Interaction: reveal a real card on mobile via the edge API.
{
  const context = await browser.newContext({
    viewport: { width: 390, height: 844 },
    isMobile: true,
    hasTouch: true,
    deviceScaleFactor: 2,
  });
  const page = await context.newPage();
  await page.goto(`${BASE}/cards`, { waitUntil: "networkidle" });
  await page.waitForTimeout(1200);
  const reveal = page.getByRole("button", { name: /Reveal Card Details/i });
  if (await reveal.isEnabled()) {
    await reveal.click();
    await page.waitForTimeout(2500);
  }
  const body = await page.locator("body").innerText();
  checks += 1;
  if (/Edge API request failed|Could not load card details/i.test(body)) {
    failures.push({ viewport: "mobile-390x844", path: "/cards (reveal)", error: "reveal failed" });
  }
  await page.screenshot({ path: `${OUT}/cards-revealed-mobile.png`, fullPage: true });
  await context.close();
}

await browser.close();
console.log(`checks: ${checks}, failures: ${failures.length}, screenshots: ${OUT}`);
if (failures.length) {
  console.log(JSON.stringify(failures, null, 2));
  process.exit(1);
}
