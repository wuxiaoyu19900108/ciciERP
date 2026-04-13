/**
 * Connect to running Chrome via CDP and update Google Sheets cells J23:J32
 * Uses #t-name-box (correct selector for Google Sheets Name Box)
 */
const playwright = require('/home/wxy/data/ciciERP/tests/node_modules/playwright');

const CELLS = ['J23','J24','J25','J26','J27','J28','J29','J30','J31','J32'];
const VALUE = '已修复 ✅';

async function navigateToCell(page, cellRef) {
  // Click the Name Box using the correct selector
  const nameBox = page.locator('#t-name-box');
  await nameBox.click({ timeout: 5000 });
  await page.waitForTimeout(200);
  await page.keyboard.press('Control+a');
  await page.keyboard.type(cellRef);
  await page.keyboard.press('Enter');
  await page.waitForTimeout(600);
}

(async () => {
  const browser = await playwright.chromium.connectOverCDP('http://localhost:37999');
  console.log('Connected to existing Chrome browser via CDP');
  
  const context = browser.contexts()[0];
  const page = context.pages()[0];
  
  console.log(`Google Sheets: ${await page.title()}`);
  
  // Wait for the Name Box to be available  
  await page.waitForSelector('#t-name-box', { timeout: 15000 });
  await page.waitForSelector('.grid-container', { timeout: 15000 });
  console.log('Sheet is ready!\n');
  await page.waitForTimeout(1000);
  
  // Press Escape to ensure no cell is in edit mode
  await page.keyboard.press('Escape');
  await page.waitForTimeout(300);
  
  // Update each cell J23 through J32
  for (const cell of CELLS) {
    process.stdout.write(`Updating ${cell}...`);
    await navigateToCell(page, cell);
    await page.keyboard.type(VALUE);
    await page.keyboard.press('Enter');
    await page.waitForTimeout(400);
    console.log(` ✓`);
  }
  
  // Wait for auto-save
  console.log('\nWaiting for auto-save (3s)...');
  await page.waitForTimeout(3000);
  
  // Screenshot
  const screenshotPath = '/home/wxy/data/ciciERP/tests/screenshots/bug028_037_update.png';
  await page.screenshot({ path: screenshotPath, fullPage: false });
  console.log(`📸 Screenshot: ${screenshotPath}`);
  
  console.log('\n✅ Done! J23:J32 set to "已修复 ✅"');
  
  await browser.close();
})().catch(err => {
  console.error('❌ Error:', err.message);
  process.exit(1);
});
