#!/usr/bin/env node
// Dedicated Chromium regression: never connects to a user's browser or database.
import assert from 'node:assert/strict';
import { spawn, execFileSync } from 'node:child_process';
import { once } from 'node:events';
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { createServer } from 'node:net';
import { dirname, resolve, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const modulePath = process.env.PLAYWRIGHT_MODULE;
assert(modulePath, 'Set PLAYWRIGHT_MODULE to an installed playwright/index.mjs');
const { chromium } = await import(pathToFileURL(resolve(modulePath)).href);
const bundle = resolve(process.env.TSUNORU_TEST_BUNDLE || join(root, 'target/dx/tsunoru/debug/web'));
const stale = process.argv.includes('--stale-css');
const label = stale ? 'stale' : process.argv.includes('--baseline') ? 'baseline' : 'verified';
const evidence = join(root, 'var/browser-evidence', label);
await mkdir(evidence, { recursive: true });
await rm(join(evidence, 'failure.png'), { force: true });
const temp = await mkdtemp(join(root, 'var/calendar-browser-'));
const socket = createServer();
socket.listen(0, '127.0.0.1');
await once(socket, 'listening');
const port = socket.address().port;
await new Promise((r) => socket.close(r));
const origin = `http://127.0.0.1:${port}`;
const env = Object.fromEntries(Object.entries(process.env).filter(([k]) => !k.startsWith('DIOXUS_') && !k.startsWith('TSUNORU_')));
const child = spawn(join(bundle, 'server'), [], { cwd: temp, env: { ...env, IP: '127.0.0.1', PORT: String(port) }, stdio: 'ignore' });
let launchError;
child.on('error', (error) => { launchError = error; });
let browser;
let browserLaunch;
let activePage;
let stopping = false;
let cleanupPromise;
function cleanup() {
  return cleanupPromise ??= (async () => {
    try {
      // A signal can arrive while Chromium is still launching.
      const ownedBrowser = browser ?? await browserLaunch;
      if (ownedBrowser) await ownedBrowser.close();
    } finally {
      if (child.exitCode === null && child.signalCode === null && child.pid) {
        const stopped = once(child, 'exit');
        child.kill('SIGTERM');
        const timer = setTimeout(() => child.kill('SIGKILL'), 5000);
        try { await stopped; } finally { clearTimeout(timer); }
      }
      await rm(temp, { recursive: true, force: true });
    }
  })();
}
for (const [signal, code] of [['SIGINT', 130], ['SIGTERM', 143]]) {
  process.on(signal, () => {
    stopping = true;
    void cleanup().then(() => process.exit(code), () => process.exit(1));
  });
}
const measurements = [];
try {
  let ready = false;
  for (let i = 0; i < 100; i++) {
    if (launchError) throw launchError;
    assert(child.exitCode === null && child.signalCode === null, 'Owned server must remain alive');
    try { ready = (await fetch(origin, { signal: AbortSignal.timeout(2000) })).ok; } catch {}
    if (ready) break;
    await new Promise((r) => setTimeout(r, 100));
  }
  assert(ready, 'Owned server should become ready');
  console.log(execFileSync('sh', [join(root, 'scripts/verify_served_calendar_assets.sh'), origin], { encoding: 'utf8', timeout: 30000 }).trim());
  if (stopping) await new Promise(() => {});
  browser = await (browserLaunch = chromium.launch({ headless: true, handleSIGINT: false, handleSIGTERM: false }));
  if (stopping) await new Promise(() => {});
  if (process.argv.includes('--shutdown-probe')) {
    console.log('shutdown_probe_ready=' + JSON.stringify({ serverPid: child.pid, temp, origin }));
    await new Promise(() => {});
  }
  for (const width of [320, 1440]) {
    const context = await browser.newContext({ viewport: { width, height: 1000 }, reducedMotion: 'reduce' });
    const page = await context.newPage();
    activePage = page;
    // A directly launched debug build has no hot-reload websocket; only application exceptions fail this check.
    page.on('response', (r) => { if (new URL(r.url()).pathname.startsWith('/api/')) console.log('HTTP', r.status(), new URL(r.url()).pathname); });
    const errors = [];
    page.on('pageerror', (e) => errors.push(e.message));
    if (stale) {
      await page.route('**/*.css', (route) => route.fulfill({ status: 200, contentType: 'text/css', body: 'body { margin: 0; } /* old bundle */' }));
    }
    const cssResponsePromise = page.waitForResponse((r) => new URL(r.url()).pathname.endsWith('.css'));
    await page.goto(origin);
    const cssResponse = await cssResponsePromise;
    const css = await cssResponse.text();
    assert.equal(cssResponse.status(), 200);
    assert.match(cssResponse.headers()['content-type'], /text\/css/);
    const missing = ['.candidate-calendar-toolbar', '.candidate-calendar-grid', '.candidate-calendar-day'].some((selector) => !css.includes(selector));
    if (stale) {
      assert(missing, 'Stale fixture must be rejected by provenance check');
      console.log('PASS stale stylesheet detected despite HTTP 200');
      await context.close();
      break;
    }
    assert(!missing, 'Served stylesheet must contain current calendar selectors');
    const linked = await page.locator('link[rel="stylesheet"]').evaluateAll((links) => links.map((link) => link.href));
    assert(linked.includes(cssResponse.url()), 'Check the actual HTML-linked stylesheet');
    const assetPath = new URL(cssResponse.url()).pathname;
    assert.match(assetPath, /^\/assets\/main-[a-z0-9]+\.css$/);
    assert.equal(css, await readFile(join(bundle, 'public', assetPath.slice(1)), 'utf8'), 'Served CSS matches this server bundle exactly');
    await page.locator('.candidate-calendar-day').first().waitFor();
    assert.equal(await page.getByLabel('候補の時刻', { exact: true }).inputValue(), '19:00');
    const geometry = await page.locator('.candidate-calendar').evaluate((calendar) => {
      const grid = calendar.querySelector('.candidate-calendar-grid');
      const style = getComputedStyle(grid);
      const rect = (el) => { const r = el.getBoundingClientRect(); return { x: r.x, y: r.y, width: r.width, height: r.height }; };
      return {
        viewport: document.documentElement.clientWidth, scrollWidth: document.documentElement.scrollWidth,
        display: style.display, columns: style.gridTemplateColumns, gap: style.gap, minWidth: style.minWidth,
        toolbar: [...calendar.querySelector('.candidate-calendar-toolbar').children].map(rect),
        targets: [...calendar.querySelectorAll('.candidate-calendar-day')].map(rect),
        labels: [...calendar.querySelectorAll('.candidate-calendar-day > span:first-child')].map(rect),
      };
    });
    measurements.push({ width, stylesheet: new URL(cssResponse.url()).pathname, geometry });
    await writeFile(join(evidence, 'measurements.json'), JSON.stringify(measurements, null, 2));
    await page.screenshot({ path: join(evidence, `calendar-${width}.png`), fullPage: true });
    assert(geometry.scrollWidth <= geometry.viewport, 'No page-level overflow');
    assert.equal(geometry.display, 'grid');
    assert.equal(geometry.columns.split(' ').length, 7, 'Seven realized calendar tracks');
    assert(geometry.targets.every((r) => r.width >= 24 && r.height >= 24), `Every day target >=24px; minimum width=${Math.min(...geometry.targets.map((r) => r.width))}`);
    const centers = geometry.toolbar.map((r) => r.y + r.height / 2);
    assert(Math.max(...centers) - Math.min(...centers) < 2, 'Toolbar stays on one row');
    assert(geometry.labels.every((r) => r.height < 25), 'Numeric date labels must not wrap');
    assert.equal(await page.locator('.candidate-calendar [role="grid"]').count(), 0, 'Native buttons must not imply an unsupported ARIA grid');
    const month = page.locator('.candidate-calendar-month');
    const initial = await month.textContent();
    await page.getByRole('button', { name: '次の月を表示' }).click();
    await page.waitForFunction((text) => document.querySelector('.candidate-calendar-month').textContent !== text, initial);
    await page.getByRole('button', { name: '前の月を表示' }).click();
    await page.waitForFunction((text) => document.querySelector('.candidate-calendar-month').textContent === text, initial);
    await page.getByRole('button', { name: '次の月を表示' }).focus();
    await page.keyboard.press('Enter');
    await page.waitForFunction((text) => document.querySelector('.candidate-calendar-month').textContent !== text, initial);
    await page.getByLabel('候補の時刻', { exact: true }).fill('20:30');
    const day = page.locator('.candidate-calendar-day').nth(9);
    await day.click();
    assert.equal(await day.getAttribute('aria-pressed'), 'true');
    assert.match(await day.getAttribute('aria-label'), /20:30/);
    await day.focus();
    await page.keyboard.press('Space');
    await page.waitForFunction(() => document.querySelectorAll('.candidate-calendar-day[aria-pressed="true"]').length === 0);
    await page.keyboard.press('Space');
    await page.waitForFunction(() => document.querySelectorAll('.candidate-calendar-day[aria-pressed="true"]').length === 1);
    await page.keyboard.press('Tab');
    assert.equal(await page.locator('.candidate-calendar-day').nth(10).evaluate((el) => el === document.activeElement), true);
    const focus = await page.locator(':focus').evaluate((el) => ({ visible: el.matches(':focus-visible'), outline: getComputedStyle(el).outlineStyle }));
    assert(focus.visible && focus.outline !== 'none', 'Keyboard focus is visible');
    assert.equal(await day.locator('.candidate-calendar-selected-mark').textContent(), '✓');
    await page.screenshot({ path: join(evidence, `selected-${width}.png`), fullPage: true });
    await page.getByLabel('イベント名', { exact: true }).fill(`Calendar browser verification ${width}`);
    await page.getByRole('button', { name: 'イベントを作る', exact: true }).click();
    const shared = page.getByRole('link', { name: '共有URLを開く', exact: true });
    await shared.waitFor();
    await shared.click();
    // SSR inputs exist before WASM attaches their listeners. Wait for Dioxus' DOM binding, not an arbitrary sleep.
    await page.locator('#respondent-name[data-dioxus-id]').waitFor();
    await page.getByLabel('あなたの名前', { exact: true }).fill('Calendar tester');
    await page.locator('.availability-option-label').filter({ hasText: '行ける' }).click();
    assert.equal(await page.locator('input[type="radio"][value="available"]').isChecked(), true);
    await page.getByRole('button', { name: '回答を送る', exact: true }).click();
    await page.getByRole('heading', { name: '回答を送りました', exact: true }).waitFor();
    await page.getByRole('heading', { name: 'みんなの回答', exact: true }).waitFor();
    assert(await page.getByText('Calendar tester', { exact: true }).count() > 0);
    assert(await page.evaluate(() => document.documentElement.scrollWidth <= document.documentElement.clientWidth), 'Post-answer page fits viewport');
    const tableScroll = page.locator('.response-matrix-scroll');
    await tableScroll.focus();
    if (await tableScroll.evaluate((el) => el.scrollWidth > el.clientWidth)) {
      await page.keyboard.press('ArrowRight');
      await page.waitForFunction(() => document.querySelector('.response-matrix-scroll').scrollLeft > 0);
    }
    await page.screenshot({ path: join(evidence, `answers-${width}.png`), fullPage: true });
    assert.deepEqual(errors, [], 'No uncaught browser errors');
    console.log(`PASS ${width}px: served CSS, seven columns, targets, keyboard, create and answer`);
    await context.close();
  }
  console.log('calendar_browser_verification=PASS');
} catch (error) {
  if (!stopping) {
    if (activePage && !activePage.isClosed()) {
      await activePage.screenshot({ path: join(evidence, 'failure.png'), fullPage: true });
      console.log('Visible alerts:', await activePage.locator('[role=alert], .form-error, .field-error').allTextContents());
    }
    throw error;
  }
} finally {
  await cleanup();
}
