import { expect, test, type Page } from '@playwright/test';
import axe from 'axe-core';

const endpoint = {
  id: 1,
  slug: 'qa-receiver',
  name: 'QA receiver',
  signature_required: true,
  created_at: '2026-08-28T00:00:00Z',
};

const highFingerprint = {
  fingerprint: 'qa-high-001',
  endpoint_id: 1,
  endpoint_name: 'QA receiver',
  event_type: 'deployment.failed',
  first_seen: '2026-08-28T00:00:00Z',
  last_seen: '2026-08-28T00:10:00Z',
  total_count: 2,
  pending_count: 2,
  severity: 'high',
  target_minutes: 30,
  acknowledged_at: null,
  overdue: false,
};

const settings = {
  quiet_start: '22:00',
  quiet_end: '08:00',
  utc_offset_minutes: 0,
  digest_minutes: 60,
  retention_days: 7,
  notification_configured: false,
  notification_url: '',
  escalation_url: '',
  last_delivery_error: null,
};

async function mockDashboardApi(page: Page): Promise<void> {
  await page.route('**/api/**', async (route) => {
    const request = route.request();
    const path = new URL(request.url()).pathname;
    if (request.method() === 'POST' && path.endsWith('/ack')) {
      await route.fulfill({ status: 204 });
      return;
    }
    if (request.method() === 'DELETE' && path.endsWith('/endpoints/1')) {
      await route.fulfill({ status: 204 });
      return;
    }
    if (request.method() === 'PATCH' && path.endsWith('/fingerprints/qa-high-001')) {
      await route.fulfill({ contentType: 'application/json', body: '{}' });
      return;
    }
    if (request.method() === 'PUT' && path.endsWith('/settings')) {
      await route.fulfill({ contentType: 'application/json', body: JSON.stringify(settings) });
      return;
    }
    if (request.method() === 'POST' && path.endsWith('/digest/send')) {
      await route.fulfill({ contentType: 'application/json', body: JSON.stringify({ sent: 0 }) });
      return;
    }
    if (path.endsWith('/summary')) {
      await route.fulfill({ contentType: 'application/json', body: JSON.stringify({ endpoints: 1, fingerprints: 1, events_today: 2, pending: 2, compressed: 1, high_unacknowledged: 1 }) });
      return;
    }
    if (path.endsWith('/endpoints')) {
      await route.fulfill({ contentType: 'application/json', body: JSON.stringify([endpoint]) });
      return;
    }
    if (path.endsWith('/fingerprints/qa-high-001')) {
      await route.fulfill({ contentType: 'application/json', body: JSON.stringify({ event_type: 'deployment.failed', payload: { type: 'deployment.failed', status: 500 }, received_at: '2026-08-28T00:10:00Z', signature_valid: true }) });
      return;
    }
    if (path.endsWith('/fingerprints')) {
      await route.fulfill({ contentType: 'application/json', body: JSON.stringify([highFingerprint]) });
      return;
    }
    await route.fulfill({ contentType: 'application/json', body: JSON.stringify(settings) });
  });
}

async function openDashboard(page: Page): Promise<void> {
  await page.addInitScript(() => sessionStorage.setItem('qh_admin_token', 'qa-token'));
  await mockDashboardApi(page);
  await page.goto('/');
  await expect(page.getByRole('heading', { level: 1, name: 'Webhook observations' })).toBeVisible();
}

for (const viewport of [{ name: 'mobile', width: 390, height: 844 }, { name: 'desktop', width: 1440, height: 900 }]) {
  test(`@regression:paid-card-contrast axe passes on authenticated Aliases at ${viewport.name}`, async ({ page }) => {
    await page.setViewportSize({ width: viewport.width, height: viewport.height });
    await openDashboard(page);
    await page.getByRole('tab', { name: /Aliases/ }).click();
    await expect(page.getByRole('button', { name: 'Restore license' })).toBeVisible();

    for (const theme of ['light', 'dark']) {
      if (theme === 'dark') await page.getByRole('button', { name: 'Toggle color theme' }).click();
      await page.addScriptTag({ content: axe.source });
      const violations = await page.evaluate(async () => {
        const report = await axe.run(document, { runOnly: ['color-contrast'] });
        return report.violations.filter((violation) => violation.impact === 'serious' || violation.impact === 'critical');
      });
      expect(violations, `${theme} ${viewport.name} Aliases panel must have no serious/critical axe contrast finding`).toEqual([]);
    }
  });
}

test('@regression:tab-roving-focus supports ArrowLeft ArrowRight Home and End', async ({ page }) => {
  await openDashboard(page);
  const observations = page.getByRole('tab', { name: 'Observations' });
  await observations.focus();
  await page.keyboard.press('ArrowRight');
  await expect(page.getByRole('tab', { name: /Aliases/ })).toBeFocused();
  await expect(page.getByRole('tab', { name: /Aliases/ })).toHaveAttribute('aria-selected', 'true');
  await expect(page.getByRole('tabpanel')).toHaveAttribute('aria-labelledby', 'tab-aliases');
  await page.keyboard.press('End');
  await expect(page.getByRole('tab', { name: 'Quiet rules' })).toBeFocused();
  await page.keyboard.press('Home');
  await expect(observations).toBeFocused();
  await page.keyboard.press('ArrowLeft');
  await expect(page.getByRole('tab', { name: 'Quiet rules' })).toBeFocused();
});

test('@regression:status-survives-dashboard-rerenders for every reported action', async ({ page }) => {
  await openDashboard(page);
  await page.evaluate(() => { (window as unknown as { stableStatus?: Element }).stableStatus = document.querySelector('#live-status')!; });
  const status = page.locator('#live-status');
  const expectStatusAfterRerender = async (message: string): Promise<void> => {
    await expect(status).toContainText(message);
    await page.waitForTimeout(300);
    await expect(status).toContainText(message);
    expect(await page.evaluate(() => document.querySelector('#live-status') === (window as unknown as { stableStatus?: Element }).stableStatus)).toBe(true);
  };

  await page.getByRole('button', { name: 'Acknowledge' }).click();
  await expectStatusAfterRerender('High-severity fingerprint acknowledged.');

  await page.getByRole('button', { name: /Inspect deployment.failed fingerprint/ }).click();
  await page.getByLabel('Signal policy').selectOption('normal');
  await page.getByRole('button', { name: 'Save rule' }).click();
  await expectStatusAfterRerender('Classification rule saved.');

  await page.getByRole('tab', { name: /Aliases/ }).click();
  page.once('dialog', (dialog) => dialog.accept());
  await page.getByRole('button', { name: 'Delete' }).click();
  await expectStatusAfterRerender('QA receiver and its retained observations were deleted.');

  await page.getByRole('tab', { name: 'Quiet rules' }).click();
  await page.getByRole('button', { name: 'Save quiet rules' }).click();
  await expectStatusAfterRerender('Quiet rules saved.');
  await page.getByRole('button', { name: 'Send digest now' }).click();
  await expectStatusAfterRerender('Nothing pending, so no digest was sent.');
});

test('@regression:mobile-persistent-links-have-44px-hit-areas', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await openDashboard(page);
  for (const locator of [page.locator('.brand'), page.getByRole('link', { name: 'Privacy' }), page.getByRole('link', { name: 'Terms' })]) {
    const box = await locator.boundingBox();
    expect(box).not.toBeNull();
    expect(box!.width).toBeGreaterThanOrEqual(44);
    expect(box!.height).toBeGreaterThanOrEqual(44);
  }
});
