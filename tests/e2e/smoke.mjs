// End-to-end smoke tests for Typst Studio (dev tooling, not run in CI).
//
// Raw Playwright (no test-runner config) so it runs with a plain `node`.
// Assertions use page.evaluate / DOM queries rather than snapshot element refs,
// which are brittle for a fine-grained-reactive WASM app. Run via ./run.sh,
// which builds + serves the app on :1420 first.
//
// Usage: BASE_URL=http://127.0.0.1:1420 node smoke.mjs

import { chromium } from 'playwright';

const BASE = process.env.BASE_URL || 'http://127.0.0.1:1420';
const results = [];

function check(name, ok, detail = '') {
  results.push({ name, ok });
  console.log(`${ok ? '✅ PASS' : '❌ FAIL'}  ${name}${detail ? ` — ${detail}` : ''}`);
}

// Wait until Leptos has mounted the editor.
async function waitForWasm(page) {
  await page.waitForSelector('textarea.typst-editor', { timeout: 20000 });
}

// Wait for the debounced (500ms) compile to produce SVG output.
async function waitForSvg(page, timeout = 15000) {
  await page.waitForSelector('.preview-content svg', { timeout });
}

async function setEditor(page, text) {
  const ta = page.locator('textarea.typst-editor');
  await ta.fill(text);
}

async function main() {
  const browser = await chromium.launch({ headless: true });
  const ctx = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await ctx.newPage();

  try {
    // ---- Scenario 1: app loads, default document renders to SVG ----
    await page.goto(BASE);
    await waitForWasm(page);
    // Start from a clean slate so the bundled default loads deterministically.
    await page.evaluate(() => localStorage.clear());
    await page.reload();
    await waitForWasm(page);
    try {
      await waitForSvg(page);
      check('1. app loads → SVG appears', true);
    } catch (e) {
      check('1. app loads → SVG appears', false, String(e).split('\n')[0]);
    }

    // ---- Scenario 2: editing re-renders ----
    await setEditor(page, '= Hello E2E\n\nSome body text.');
    try {
      // New compile: wait for an SVG and no error alert.
      await page.waitForTimeout(900); // debounce + compile
      await waitForSvg(page);
      const hasError = await page.locator('[role="alert"]').count();
      check('2. type = Hello → re-render', hasError === 0);
    } catch (e) {
      check('2. type = Hello → re-render', false, String(e).split('\n')[0]);
    }

    // ---- Scenario 3: error reports the user's editor line ----
    await setEditor(page, '= Title\n#undefined_fn()');
    try {
      const alert = page.locator('[role="alert"]');
      await alert.waitFor({ state: 'visible', timeout: 8000 });
      const txt = await alert.innerText();
      check('3. error on line 2 contains "2:"', txt.includes('2:'), txt.replace(/\s+/g, ' ').slice(0, 80));
    } catch (e) {
      check('3. error on line 2 contains "2:"', false, String(e).split('\n')[0]);
    }

    // ---- Scenario 4: multi-file tab isolation ----
    try {
      await setEditor(page, 'MAIN FILE BODY');
      await page.waitForTimeout(300);
      // Create a second file.
      await page.locator('button[aria-label="New file"]').click();
      await page.getByRole('button', { name: 'Create', exact: true }).click();
      await page.waitForTimeout(200);
      await setEditor(page, 'SECOND FILE BODY');
      await page.waitForTimeout(300);
      // Switch back to the first tab.
      await page.locator('text=main.typ').first().click();
      await page.waitForTimeout(200);
      const firstVal = await page.locator('textarea.typst-editor').inputValue();
      const isolated = firstVal.includes('MAIN FILE BODY') && !firstVal.includes('SECOND FILE BODY');
      check('4. tab create/switch isolation', isolated, `first tab = "${firstVal.slice(0, 30)}"`);
    } catch (e) {
      check('4. tab create/switch isolation', false, String(e).split('\n')[0]);
    }

    // ---- Scenario 5: share-link #src= roundtrip ----
    try {
      const src = '= Shared Document\n\nFrom the URL fragment.';
      // URL_SAFE_NO_PAD base64 (matches utils/share.rs).
      const b64 = Buffer.from(src, 'utf8')
        .toString('base64')
        .replace(/\+/g, '-')
        .replace(/\//g, '_')
        .replace(/=+$/, '');
      await page.goto(`${BASE}/#src=${b64}`);
      // A fragment-only change doesn't reload the page, so the WASM app never
      // re-reads the hash. Force a reload (which preserves the fragment).
      await page.reload();
      await waitForWasm(page);
      await page.waitForTimeout(300);
      const val = await page.locator('textarea.typst-editor').inputValue();
      check('5. share-link #src= roundtrip', val.includes('Shared Document'), `editor = "${val.slice(0, 30)}"`);
    } catch (e) {
      check('5. share-link #src= roundtrip', false, String(e).split('\n')[0]);
    }
  } finally {
    await browser.close();
  }

  const failed = results.filter((r) => !r.ok).length;
  console.log(`\n${results.length - failed}/${results.length} scenarios passed`);
  process.exit(failed === 0 ? 0 : 1);
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
