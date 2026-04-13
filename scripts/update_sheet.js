/**
 * Script to update Google Spreadsheet column J (修复状态) for BUG-028 to BUG-037
 * Rows 23-32, cells J23:J32 → "已修复 ✅"
 */
const { chromium } = require('/home/wxy/data/ciciERP/tests/node_modules/playwright');

const SPREADSHEET_URL = 'https://docs.google.com/spreadsheets/d/1uM0vKs-rJOg7rUP46TRQKSl7AW7N59NszSAQEcI-pmc/edit';
const VALUE = '已修复 ✅';
const CELLS = ['J23','J24','J25','J26','J27','J28','J29','J30','J31','J32'];

async function updateCell(page, cellRef, value) {
  console.log(`Updating ${cellRef}...`);

  // Click the Name Box (top-left cell reference input)
  const nameBox = page.locator('.cell-input').or(
    page.locator('[aria-label="Name Box"]')
  ).or(
    page.locator('.goog-inline-block.docs-spreadsheet-name-box-input')
  ).first();

  // Use keyboard shortcut Ctrl+G or just click the name box
  // The name box in Google Sheets is typically the first input in the toolbar area
  await page.locator('.cell-input').first().click({ force: true }).catch(async () => {
    // fallback: try clicking the name box area using coordinates approach
    // Google Sheets name box is at top-left
    await page.keyboard.press('Escape');
    await page.keyboard.press('Control+Home');
  });

  // Try to find and use the Name Box
  // In Google Sheets the name box has aria-label "Name Box"  
  const nameBoxLocator = page.locator('[aria-label="Name Box"]');
  await nameBoxLocator.click();
  await page.keyboard.press('Control+a');
  await page.keyboard.type(cellRef);
  await page.keyboard.press('Enter');
  await page.waitForTimeout(500);

  // Now type the value and press Enter
  await page.keyboard.type(value);
  await page.keyboard.press('Enter');
  await page.waitForTimeout(400);

  console.log(`  ✓ ${cellRef} = "${value}"`);
}

(async () => {
  const browser = await chromium.launch({ 
    headless: false,
    args: ['--no-sandbox', '--disable-setuid-sandbox']
  });
  
  // Use persistent context to reuse saved Google login session
  const userDataDir = '/home/wxy/.config/google-chrome';
  let context;
  
  try {
    context = await chromium.launchPersistentContext(userDataDir, {
      headless: false,
      args: ['--no-sandbox', '--disable-setuid-sandbox'],
    });
    console.log('Using persistent Chrome profile');
  } catch (e) {
    console.log('Falling back to new browser context:', e.message);
    context = await browser.newContext();
  }

  const page = context.pages()[0] || await context.newPage();

  console.log('Navigating to spreadsheet...');
  await page.goto(SPREADSHEET_URL);

  // Wait for .waffle element (as instructed, NOT networkidle)
  console.log('Waiting for .waffle element...');
  await page.waitForSelector('.waffle', { timeout: 30000 });
  console.log('Spreadsheet loaded!');
  await page.waitForTimeout(2000);

  // Update each cell
  for (const cell of CELLS) {
    await updateCell(page, cell, VALUE);
  }

  // Final wait to ensure saves are persisted
  await page.waitForTimeout(3000);
  console.log('\n✅ All cells updated successfully!');
  console.log('Cells J23:J32 set to "已修复 ✅"');

  await context.close();
})().catch(err => {
  console.error('Error:', err);
  process.exit(1);
});
