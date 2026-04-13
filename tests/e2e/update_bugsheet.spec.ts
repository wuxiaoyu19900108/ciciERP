/**
 * 更新 Google 表格 BUG-007 ~ BUG-027 修复状态 (列 J "修复状态")
 */
import { test } from '@playwright/test';
import * as path from 'path';
import * as fs from 'fs';

const SHEET_URL = 'https://docs.google.com/spreadsheets/d/1uM0vKs-rJOg7rUP46TRQKSl7AW7N59NszSAQEcI-pmc/edit';

const BUG_UPDATES: Record<string, string> = {
  'BUG-007': '已修复 ✅',
  'BUG-008': '已修复 ✅',
  'BUG-009': '已修复 ✅',
  'BUG-010': '已修复 ✅',
  'BUG-011': '已修复 ✅',
  'BUG-012': '已修复 ✅',
  'BUG-013': '已修复 ✅',
  'BUG-014': '已修复 ✅',
  'BUG-015': '已修复 ✅',
  'BUG-016': '已修复 ✅',
  'BUG-017': '待修复 (P2)',
  'BUG-018': '已修复 ✅',
  'BUG-019': '已修复 ✅',
  'BUG-020': '已修复 ✅',
  'BUG-021': '已修复 ✅',
  'BUG-022': '已修复 ✅',
  'BUG-023': '待修复 (P2)',
  'BUG-024': '已修复 ✅',
  'BUG-025': '已修复 ✅',
  'BUG-026': '已修复 ✅',
  'BUG-027': '已修复 ✅',
};

const screenshotDir = path.join(__dirname, '..', 'screenshots');

async function saveScreenshot(page: any, name: string) {
  if (!fs.existsSync(screenshotDir)) fs.mkdirSync(screenshotDir, { recursive: true });
  const p = path.join(screenshotDir, name);
  await page.screenshot({ path: p, fullPage: false });
  console.log(`📸 Screenshot: ${p}`);
}

/** Navigate to a cell via Name Box */
async function goToCell(page: any, cellRef: string) {
  // Try multiple selectors for the Name Box
  const nameBoxSelectors = [
    '#t-name-box',
    '.t-name-box',
    '[aria-label="Name Box"]',
    '[aria-label="Cell reference"]',
    '.goog-flat-menu-button-caption',
  ];

  let nameBoxFound = false;
  for (const sel of nameBoxSelectors) {
    const el = page.locator(sel).first();
    if (await el.count() > 0) {
      await el.click({ timeout: 3000 }).catch(() => null);
      nameBoxFound = true;
      break;
    }
  }

  if (!nameBoxFound) {
    // Use keyboard shortcut Ctrl+G or Ctrl+J won't work; try clicking the formula bar area
    // As fallback: use the Name Box via its position (top-left of the spreadsheet)
    await page.keyboard.press('Escape');
    await page.waitForTimeout(200);
    // Try clicking the Name Box via coordinate (it's typically at top-left ~85px from left)
    await page.mouse.click(50, 100);
  }

  await page.waitForTimeout(300);
  await page.keyboard.press('Control+a');
  await page.keyboard.type(cellRef);
  await page.keyboard.press('Enter');
  await page.waitForTimeout(500);
}

test('更新 BUG 修复状态 (J 列)', async ({ page }) => {
  test.setTimeout(300000); // 5 minutes

  // Step 1: Navigate
  console.log('🌐 Navigating to Google Sheets...');
  await page.goto(SHEET_URL, { timeout: 60000 });

  // Step 2: Wait for sheet to load - NEVER use networkidle with Google Sheets
  console.log('⏳ Waiting for .waffle selector...');
  try {
    await page.waitForSelector('.waffle', { timeout: 30000 });
    console.log('✅ Sheet loaded (.waffle found)');
  } catch {
    console.log('⚠️  .waffle not found, checking page state...');
    const url = page.url();
    const title = await page.title();
    console.log(`URL: ${url}, Title: ${title}`);
    await saveScreenshot(page, '00_load_error.png');

    if (url.includes('accounts.google.com')) {
      console.log('❌ Google login required - cannot proceed');
      return;
    }
    // Try waiting longer
    await page.waitForTimeout(5000);
  }

  // Step 3: Screenshot current state
  await saveScreenshot(page, '01_before.png');
  console.log(`Current URL: ${page.url()}`);

  // Step 4: Find bug row numbers using JavaScript
  console.log('🔍 Finding bug row indices...');
  await page.waitForTimeout(2000); // Let the sheet fully render

  const bugRows: Record<string, number> = await page.evaluate(() => {
    const result: Record<string, number> = {};
    const rows = document.querySelectorAll('.waffle tbody tr, .waffle tr');
    rows.forEach((row, rowIdx) => {
      const cells = row.querySelectorAll('td');
      cells.forEach(cell => {
        const text = cell.textContent?.trim() || '';
        if (/^BUG-\d+$/.test(text)) {
          result[text] = rowIdx + 1; // 1-based row index in the visible sheet
        }
      });
    });
    return result;
  });

  console.log('Found bug rows:', JSON.stringify(bugRows));

  if (Object.keys(bugRows).length === 0) {
    console.log('⚠️  No bug rows found via JS, will try sequential row approach');
    // Fallback: assume header is row 1, bugs start at row 2
    for (let i = 0; i < 21; i++) {
      const bugId = `BUG-${String(7 + i).padStart(3, '0')}`;
      bugRows[bugId] = i + 2; // rows 2..22
    }
  }

  // Step 5: Update each cell in column J
  // First, determine the actual spreadsheet row numbers
  // The waffle rows include frozen/header rows, so we need to map carefully
  // Let's also check header row to find column J position
  const colInfo = await page.evaluate(() => {
    // Find the header row
    const rows = document.querySelectorAll('.waffle tbody tr, .waffle tr');
    let headerRowIdx = -1;
    let jColIdx = -1;
    rows.forEach((row, rowIdx) => {
      const cells = row.querySelectorAll('td');
      cells.forEach((cell, colIdx) => {
        const text = cell.textContent?.trim() || '';
        if (text === '修复状态' || text.includes('修复状态')) {
          headerRowIdx = rowIdx;
          jColIdx = colIdx;
        }
      });
    });
    return { headerRowIdx, jColIdx };
  });

  console.log('Column info:', JSON.stringify(colInfo));

  // Navigate to sheet and update cells using the Name Box approach
  // We use the Name Box to navigate to J{row} for each bug
  
  let updatedCount = 0;
  
  for (const [bugId, status] of Object.entries(BUG_UPDATES)) {
    const sheetRow = bugRows[bugId];
    if (!sheetRow) {
      console.log(`⚠️  Row not found for ${bugId}, skipping`);
      continue;
    }

    console.log(`📝 Updating ${bugId} (row ${sheetRow}) → "${status}"`);

    // Navigate to J{row} via Name Box
    await goToCell(page, `J${sheetRow}`);
    await page.waitForTimeout(300);

    // Type the status (cell is in edit mode after navigation)
    await page.keyboard.type(status);
    await page.keyboard.press('Enter'); // Confirm and move to next row
    await page.waitForTimeout(200);

    updatedCount++;
  }

  console.log(`\n✅ Updated ${updatedCount} cells`);

  // Step 6: Final screenshot
  await page.waitForTimeout(3000);
  await saveScreenshot(page, '02_after.png');
  console.log('✅ Done! Check screenshots/02_after.png for results');
});
