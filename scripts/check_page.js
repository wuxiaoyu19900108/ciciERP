const playwright = require('/home/wxy/data/ciciERP/tests/node_modules/playwright');

(async () => {
  const browser = await playwright.chromium.connectOverCDP('http://localhost:37999');
  const context = browser.contexts()[0];
  const page = context.pages()[0];
  
  console.log('URL:', page.url());
  console.log('Title:', await page.title());
  
  // Check what elements exist on the page
  const result = await page.evaluate(() => {
    const selectors = [
      '.waffle', '#waffle', '.grid-container', '.cell-canvas',
      '[class*="waffle"]', '[id*="waffle"]', '.docs-spreadsheet',
      '.grid', 'canvas', '.cell-input', '[aria-label="Name Box"]',
      '#t-name-box', '.goog-flat-menu-button', '.app-container',
      '[data-cell-id]', '.cell', '.frozen-rows'
    ];
    const found = {};
    for (const sel of selectors) {
      const el = document.querySelector(sel);
      found[sel] = el ? `YES (${el.tagName}.${el.className.substring(0,50)})` : 'NO';
    }
    return found;
  });
  
  console.log('\nElements found:');
  for (const [sel, val] of Object.entries(result)) {
    if (val !== 'NO') console.log(`  ${sel}: ${val}`);
  }
  
  // Also get page HTML structure
  const bodyClass = await page.evaluate(() => document.body.className);
  console.log('\nbody class:', bodyClass.substring(0, 200));
  
  await browser.close();
})().catch(err => { console.error(err.message); process.exit(1); });
