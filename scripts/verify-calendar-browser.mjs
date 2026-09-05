#!/usr/bin/env node
// Dedicated Chromium regression: never connects to a user's browser or database.
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { createServer } from 'node:net';
import { dirname, resolve, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { createIdentityDatabase, checkDatabaseIdentity } from './verification-database.mjs';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const modulePath = process.env.PLAYWRIGHT_MODULE;
assert(modulePath, 'Set PLAYWRIGHT_MODULE to an installed playwright/index.mjs');
const { chromium } = await import(pathToFileURL(resolve(modulePath)).href);
const bundle = resolve(process.env.TSUNORU_TEST_BUNDLE || join(root, 'target/dx/tsunoru/debug/web'));
const stale = process.argv.includes('--stale-css');
const label = stale ? 'stale' : process.argv.includes('--baseline') ? 'baseline' : 'verified';
const evidence = join(root, 'var/browser-evidence', label);
let temp, tempCreation, socket, socketSetup, socketClosing, child, origin;
let launchError, browser, browserLaunch, activePage;
let databaseIdentity;
let assetChild, assetDone, assetStopPromise;
let assetFinished = false;
let stopping = false;
let cleanupPromise;
function assertOwnedServer() {
  if (launchError) throw launchError;
  assert(child?.pid && child.exitCode === null && child.signalCode === null,
    'Owned server exited; refusing test traffic');
}
async function verifyIdentity() {
  assertOwnedServer();
  await checkDatabaseIdentity(origin, databaseIdentity);
  assertOwnedServer();
}
async function checkpoint() {
  if (stopping) await new Promise(() => {});
}
async function closeSocket() {
  if (!socket) return;
  await socketSetup.catch(() => {});
  return socketClosing ??= new Promise((done) => socket.close(() => done()));
}
function signalAssetGroup(signal) {
  if (!assetChild?.pid) return;
  try { process.kill(-assetChild.pid, signal); } catch (error) { if (error.code !== 'ESRCH') throw error; }
}
function stopAsset() {
  if (!assetChild || assetFinished) return Promise.resolve();
  return assetStopPromise ??= (async () => {
    signalAssetGroup('SIGTERM');
    const timer = setTimeout(() => signalAssetGroup('SIGKILL'), 5000);
    try { await assetDone; } finally { clearTimeout(timer); }
  })();
}
async function closeBrowser() {
  let timer;
  // Leave the close continuation attached: a launch that settles during other
  // resource cleanup must still close its browser rather than start tests.
  const closing = (async () => {
    const ownedBrowser = browser ?? await browserLaunch;
    if (ownedBrowser) await ownedBrowser.close();
  })();
  try {
    await Promise.race([closing, new Promise((_, fail) => {
      timer = setTimeout(() => fail(new Error('Browser cleanup exceeded 6 seconds')), 6000);
    })]);
  } finally {
    clearTimeout(timer);
  }
}
function cleanup() {
  return cleanupPromise ??= (async () => {
    try {
      const results = await Promise.allSettled([
        stopAsset(),
        closeBrowser(),
      ]);
      const failed = results.find((result) => result.status === 'rejected');
      if (failed) throw failed.reason;
    } finally {
      try {
        if (child && child.exitCode === null && child.signalCode === null && child.pid) {
          const stopped = once(child, 'exit');
          child.kill('SIGTERM');
          const timer = setTimeout(() => child.kill('SIGKILL'), 5000);
          try { await stopped; } finally { clearTimeout(timer); }
        }
      } finally {
        await closeSocket();
        // Account for a signal while mkdtemp is still pending.
        const directory = temp ?? await tempCreation;
        if (directory) await rm(directory, { recursive: true, force: true });
      }
    }
  })();
}
for (const [signal, code] of [['SIGINT', 130], ['SIGTERM', 143]]) {
  process.on(signal, () => {
    stopping = true;
    void cleanup().then(() => process.exit(code), (error) => { console.error(error.message); process.exit(1); });
  });
}
async function shutdownProbe(stage) {
  await checkpoint();
  if (process.argv.includes(`--shutdown-probe=${stage}`)) {
    console.log('shutdown_probe_ready=' + JSON.stringify({ serverPid: child?.pid ?? null, temp, origin }));
    // Early startup phases need a live handle while the regression delivers its signal.
    setInterval(() => {}, 1000);
    await new Promise(() => {});
  }
}
const measurements = [];
try {
  await mkdir(evidence, { recursive: true });
  await rm(join(evidence, 'failure.png'), { force: true });
  await checkpoint();
  temp = await (tempCreation = mkdtemp(join(root, 'var/calendar-browser-')));
  await shutdownProbe('temp');
  databaseIdentity = createIdentityDatabase(temp);
  socket = createServer();
  socketSetup = new Promise((done, fail) => {
    socket.once('error', fail);
    socket.listen(0, '127.0.0.1', done);
  });
  await shutdownProbe('socket');
  await socketSetup;
  const port = socket.address().port;
  await closeSocket();
  await checkpoint();
  origin = `http://127.0.0.1:${port}`;
  const env = Object.fromEntries(Object.entries(process.env).filter(([k]) => !k.startsWith('DIOXUS_') && !k.startsWith('TSUNORU_')));
  child = spawn(join(bundle, 'server'), [], { cwd: temp, env: { ...env, IP: '127.0.0.1', PORT: String(port) }, stdio: 'ignore' });
  child.on('error', (error) => { launchError = error; });
  child.once('exit', () => { if (browser) void browser.close().catch(() => {}); });
  await shutdownProbe('server');
  let ready = false;
  for (let i = 0; i < 100; i++) {
    if (launchError) throw launchError;
    assert(child.exitCode === null && child.signalCode === null, 'Owned server must remain alive');
    try { ready = (await fetch(origin, { signal: AbortSignal.timeout(2000) })).ok; } catch {}
    if (ready) break;
    await new Promise((r) => setTimeout(r, 100));
  }
  assert(ready, 'Owned server should become ready');
  await verifyIdentity();
  const checker = process.argv.includes('--shutdown-probe=asset')
    ? 'scripts/fixtures/stalled-calendar-assets.sh' : 'scripts/verify_served_calendar_assets.sh';
  assetChild = spawn('sh', [join(root, checker), origin], {
    detached: true, stdio: ['ignore', 'pipe', 'pipe'],
  });
  let assetOutput = '', assetError = '';
  assetChild.stdout.on('data', (data) => { assetOutput += data; });
  assetChild.stderr.on('data', (data) => { assetError += data; });
  assetDone = new Promise((done) => {
    assetChild.once('error', (error) => done({ error }));
    assetChild.once('close', (code) => { assetFinished = true; done({ code }); });
  });
  await shutdownProbe('asset');
  let assetTimedOut = false;
  const assetTimer = setTimeout(() => { assetTimedOut = true; void stopAsset(); }, 30000);
  let assetResult;
  try { assetResult = await assetDone; } finally { clearTimeout(assetTimer); }
  assert(!assetTimedOut && !assetResult.error && assetResult.code === 0,
    `Asset check failed: ${assetResult.error?.message || assetError || assetResult.code}`);
  console.log(assetOutput.trim());
  await checkpoint();
  browserLaunch = chromium.launch({ headless: true, timeout: 5000, handleSIGINT: false, handleSIGTERM: false });
  // A probe may pause before the launch promise is awaited.
  void browserLaunch.catch(() => {});
  await shutdownProbe('pending-browser');
  await shutdownProbe('pending-process');
  browser = await browserLaunch;
  await shutdownProbe('browser');
  for (const width of [320, 1440]) {
    const context = await browser.newContext({ viewport: { width, height: 1000 }, reducedMotion: 'reduce' });
    await context.route('**/*', async (route) => {
      try {
        assertOwnedServer();
        if (!['GET', 'HEAD'].includes(route.request().method())) await verifyIdentity();
        await route.continue();
      } catch {
        await route.abort('failed').catch(() => {});
      }
    });
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
