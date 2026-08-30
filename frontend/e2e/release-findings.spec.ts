import { expect, test } from '@playwright/test';
import { readFile } from 'node:fs/promises';
import axe from 'axe-core';

test('@claim:demo-sandbox opens an isolated seeded workspace and resets it', async ({ page, request }) => {
  const demoRequests: Array<{ url: string; authorization?: string }> = [];
  page.on('request', (outgoing) => {
    if (outgoing.url().includes('/api/demo/')) {
      demoRequests.push({ url: outgoing.url(), authorization: outgoing.headers().authorization });
    }
  });
  await page.goto('/demo');

  await expect(page.getByRole('heading', { level: 1, name: 'Sample webhook observations' })).toBeVisible();
  await expect(page.getByLabel('Demo status')).toContainText('Demo — sample data, nothing is saved');
  await expect(page.getByText('Deploy monitor').first()).toBeVisible();
  expect(demoRequests.length).toBeGreaterThan(0);
  expect(demoRequests.every((entry) => !entry.authorization)).toBe(true);
  const provisionRequests = demoRequests.filter((entry) => entry.url.endsWith('/api/demo/session')).length;

  await page.getByRole('button', { name: 'Acknowledge' }).click();
  await expect(page.getByText('High awaiting ack').locator('..').locator('strong')).toHaveText('0');
  await page.reload();
  await expect(page.getByText('High awaiting ack').locator('..').locator('strong')).toHaveText('0');
  expect(demoRequests.filter((entry) => entry.url.endsWith('/api/demo/session'))).toHaveLength(provisionRequests);

  await page.getByRole('button', { name: 'Reset demo' }).click();
  await expect(page.getByText('High awaiting ack').locator('..').locator('strong')).toHaveText('1');

  const real = await request.get('/api/summary', { headers: { Authorization: 'Bearer qa-token' } });
  expect(real.ok()).toBe(true);
  expect((await real.json()).events_today).toBe(0);

  const workspace = await page.evaluate(() => sessionStorage.getItem('demo:webhook-quiet-hours:workspace'));
  await page.getByRole('button', { name: 'Start for real' }).click();
  await expect(page).toHaveURL(/\/$/);
  await expect(page.getByRole('heading', { level: 1, name: 'Group webhook failures before they reach Slack' })).toBeVisible();
  const discarded = await request.get(`/api/demo/${workspace}/summary`);
  expect(discarded.status()).toBe(404);
});

test('@claim:repeat-compression sample data proves repeated deliveries are grouped', async ({ page }) => {
  await page.goto('/demo');
  await expect(page.getByText('Observed today').locator('..').locator('strong')).toHaveText('18');
  await expect(page.getByText('Fingerprints', { exact: true }).locator('..').locator('strong')).toHaveText('3');
  await expect(page.getByText('Repeats compressed').locator('..').locator('strong')).toHaveText('15');
  await expect(page.locator('.ledger > li')).toHaveCount(3);
});

test('@claim:csv-export exports one CSV row for each sample fingerprint', async ({ page }) => {
  await page.goto('/demo');
  const [download] = await Promise.all([
    page.waitForEvent('download'),
    page.getByRole('button', { name: 'Export CSV' }).click(),
  ]);
  expect(download.suggestedFilename()).toBe('webhook-fingerprints-demo.csv');
  const path = await download.path();
  expect(path).not.toBeNull();
  const csv = await readFile(path!, 'utf8');
  const rows = csv.trim().split('\n');
  expect(rows[0]).toBe('alias,fingerprint,event_type,severity,total_count,pending_count,first_seen,last_seen,acknowledged_at');
  expect(rows).toHaveLength(4);
  expect(csv).toContain('deployment.failed');
});

test('@claim:privacy-same-origin demo flow sends no cross-origin requests', async ({ page }) => {
  const origins = new Set<string>();
  page.on('request', (request) => origins.add(new URL(request.url()).origin));
  await page.goto('/demo');
  await page.getByRole('button', { name: /Inspect deployment.failed fingerprint/ }).click();
  await expect(page.getByText(/"service": "checkout-api"/)).toBeVisible();
  await page.getByRole('tab', { name: 'Quiet rules' }).click();
  await page.getByRole('button', { name: 'Send digest now' }).click();
  expect([...origins]).toEqual(['http://127.0.0.1:4173']);
});

test('@claim:one-time-price keeps core demo actions open and states the exact optional price', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByRole('heading', { name: 'Add aliases and longer retention' })).toBeVisible();
  await expect(page.getByRole('link', { name: 'Buy Field Station for $39 once' })).toBeVisible();
  await page.goto('/demo');
  expect(await page.evaluate(() => localStorage.getItem('sb_license:webhook-quiet-hours'))).toBeNull();
  await expect(page.getByRole('button', { name: 'Export CSV' })).toBeEnabled();
  await expect(page.getByRole('button', { name: 'Acknowledge' })).toBeEnabled();
  await page.getByRole('tab', { name: /Aliases/ }).click();
  await expect(page.getByText('One-time purchase · $39')).toBeVisible();
  await expect(page.getByRole('link', { name: 'Buy for $39 once' })).toHaveAttribute('href', /api\.sociobot\.in\/api\/v1\/products\/webhook-quiet-hours\/checkout/);
});

for (const viewport of [{ name: 'mobile', width: 390, height: 844 }, { name: 'desktop', width: 1440, height: 900 }]) {
  test(`landing and demo pass accessibility at ${viewport.name}`, async ({ page }) => {
    await page.setViewportSize(viewport);
    for (const route of ['/', '/demo']) {
      await page.goto(route);
      for (const theme of ['light', 'dark']) {
        if (theme === 'dark') await page.getByRole('button', { name: 'Toggle color theme' }).click();
        await page.addScriptTag({ content: axe.source });
        const violations = await page.evaluate(async () => {
          const report = await axe.run(document);
          return report.violations.filter((violation) => violation.impact === 'serious' || violation.impact === 'critical');
        });
        expect(violations, `${route} in ${theme} at ${viewport.name}`).toEqual([]);
      }
      await page.getByRole('button', { name: 'Toggle color theme' }).click();
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= document.documentElement.clientWidth)).toBe(true);
    }
  });
}

test('plain first screen and keyboard path lead directly to sample data', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto('/');
  await expect(page.getByRole('heading', { level: 1, name: 'Group webhook failures before they reach Slack' })).toBeVisible();
  await expect(page.getByText(/For small engineering teams/)).toBeVisible();
  const action = page.getByRole('link', { name: 'Try it with sample data' });
  await action.focus();
  await expect(action).toBeFocused();
  await page.keyboard.press('Enter');
  await expect(page).toHaveURL(/\/demo$/);
  await expect(page.getByRole('heading', { level: 1, name: 'Sample webhook observations' })).toBeVisible();
});

test('paid inline link and demo controls meet 44px touch target minimum', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto('/demo');
  await page.getByRole('tab', { name: /Aliases/ }).click();
  for (const target of [
    page.getByRole('link', { name: 'Buy for $39 once' }),
    page.getByRole('button', { name: 'Reset demo' }),
    page.getByRole('button', { name: 'Start for real' }),
  ]) {
    const box = await target.boundingBox();
    expect(box).not.toBeNull();
    expect(box!.height).toBeGreaterThanOrEqual(44);
    expect(box!.width).toBeGreaterThanOrEqual(44);
  }
});

test('license dialog manages keyboard focus and reduced motion removes transforms', async ({ page }) => {
  await page.addInitScript(() => sessionStorage.setItem('qh_admin_token', 'qa-token'));
  await page.route('**/api/**', async (route) => {
    const path = new URL(route.request().url()).pathname;
    if (path.endsWith('/summary')) return route.fulfill({ json: { endpoints: 1, fingerprints: 0, events_today: 0, pending: 0, compressed: 0, high_unacknowledged: 0 } });
    if (path.endsWith('/endpoints')) return route.fulfill({ json: [{ id: 1, slug: 'keyboard', name: 'Keyboard source', signature_required: true, created_at: '2026-08-30T00:00:00Z' }] });
    if (path.endsWith('/fingerprints')) return route.fulfill({ json: [] });
    return route.fulfill({ json: { quiet_start: '22:00', quiet_end: '08:00', utc_offset_minutes: 0, digest_minutes: 60, retention_days: 7, notification_configured: false, notification_url: '', escalation_url: '', last_delivery_error: null } });
  });
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto('/');
  await page.getByRole('tab', { name: /Aliases/ }).click();
  const restore = page.getByRole('button', { name: 'Restore license' });
  await restore.focus();
  await page.keyboard.press('Enter');
  await expect(page.getByRole('dialog')).toBeVisible();
  await expect(page.getByLabel('License token')).toBeFocused();
  await page.keyboard.press('Escape');
  await expect(page.getByRole('dialog')).not.toBeVisible();
  await expect(restore).toBeFocused();
  expect(await page.evaluate(() => ({
    reduced: matchMedia('(prefers-reduced-motion: reduce)').matches,
    scrollBehavior: getComputedStyle(document.documentElement).scrollBehavior,
  }))).toEqual({ reduced: true, scrollBehavior: 'auto' });
});

test('discovery metadata and the designed 404 route are real responses', async ({ page, request }) => {
  const robots = await request.get('/robots.txt');
  expect(robots.status()).toBe(200);
  expect(robots.headers()['content-type']).toContain('text/plain');
  expect(await robots.text()).toContain('Sitemap: https://webhook-quiet-hours.sociobot.in/sitemap.xml');

  const sitemap = await request.get('/sitemap.xml');
  expect(sitemap.status()).toBe(200);
  expect(sitemap.headers()['content-type']).toMatch(/xml/);
  expect(await sitemap.text()).toContain('<loc>https://webhook-quiet-hours.sociobot.in/demo</loc>');

  await page.goto('/');
  await expect(page.locator('link[rel="canonical"]')).toHaveAttribute('href', 'https://webhook-quiet-hours.sociobot.in/');
  await expect(page.locator('meta[property="og:image"]')).toHaveAttribute('content', /og-image\.webp$/);
  await expect(page.locator('meta[name="twitter:card"]')).toHaveAttribute('content', 'summary_large_image');
  await expect(page.locator('link[rel="apple-touch-icon"]')).toHaveAttribute('href', '/apple-touch-icon.png');

  const response = await page.goto('/missing-field-note');
  expect(response?.status()).toBe(404);
  await expect(page.getByRole('heading', { level: 1, name: 'This page is not in the field log' })).toBeVisible();
  await expect(page.getByRole('link', { name: 'Return home' })).toBeVisible();
});

test('versioned service worker precaches built assets and supports an offline reload', async ({ browser }) => {
  const context = await browser.newContext({ serviceWorkers: 'allow' });
  const page = await context.newPage();
  await page.goto('/');
  await page.evaluate(async () => {
    await navigator.serviceWorker.ready;
    if (!navigator.serviceWorker.controller) await new Promise<void>((resolve) => navigator.serviceWorker.addEventListener('controllerchange', () => resolve(), { once: true }));
  });
  const cache = await page.evaluate(async () => {
    const names = await caches.keys();
    const active = names.find((name) => name.startsWith('quiet-hours-shell-')) || '';
    const requests = active ? await (await caches.open(active)).keys() : [];
    return { active, paths: requests.map((request) => new URL(request.url).pathname) };
  });
  expect(cache.active).toMatch(/^quiet-hours-shell-[a-f0-9]{12}$/);
  expect(cache.paths.some((path) => /\/assets\/index-[A-Za-z0-9_-]+\.js$/.test(path))).toBe(true);
  expect(cache.paths.some((path) => /\/assets\/index-[A-Za-z0-9_-]+\.css$/.test(path))).toBe(true);
  await context.setOffline(true);
  await page.reload();
  await expect(page.getByRole('heading', { level: 1, name: 'Group webhook failures before they reach Slack' })).toBeVisible();
  await context.close();
});
