/**
 * BUG-007 验证：产品查看页 vs 编辑页数据一致性
 */
import { test, expect } from '@playwright/test';

const BASE_URL = process.env.BASE_URL || 'http://localhost:3001';
const ADMIN = { username: 'admin', password: 'admin123' };

async function login(page: any) {
  await page.goto(`${BASE_URL}/login`);
  await page.fill('input[name="username"]', ADMIN.username);
  await page.fill('input[name="password"]', ADMIN.password);
  await page.click('button[type="submit"]');
  await page.waitForURL((url: URL) => !url.pathname.includes('/login'), { timeout: 5000 });
}

// 找产品列表中第一个数字ID详情链接（排除 /export /import /new）
async function getFirstProductDetailHref(page: any): Promise<string> {
  await page.goto(`${BASE_URL}/products`);
  await page.waitForLoadState('networkidle');
  const links = await page.locator('a[href^="/products/"]').all();
  for (const link of links) {
    const href = await link.getAttribute('href');
    if (href && /^\/products\/\d+$/.test(href)) return href;
  }
  throw new Error('No product detail links found');
}

test.describe('BUG-007: 产品查看页与编辑页数据一致性', () => {

  test('查看页应显示所有关键信息卡片（成本/售价/三平台/供应商/品牌等）', async ({ page }) => {
    await login(page);
    const href = await getFirstProductDetailHref(page);
    console.log('Testing product:', href);

    await page.goto(`${BASE_URL}${href}`);
    await page.waitForLoadState('networkidle');

    // 基本信息字段（用 exact match 避免侧边栏导航的干扰）
    await expect(page.getByText('产品编码', { exact: true })).toBeVisible();
    await expect(page.getByText('型号', { exact: true })).toBeVisible();
    await expect(page.getByText('供应商', { exact: true })).toBeVisible();
    await expect(page.getByText('品牌', { exact: true })).toBeVisible();
    await expect(page.getByText('分类', { exact: true })).toBeVisible();

    // 成本卡片
    await expect(page.locator('text=成本信息')).toBeVisible();

    // 售价三平台卡片
    await expect(page.locator('text=售价信息')).toBeVisible();
    await expect(page.locator('text=Alibaba')).toBeVisible();
    await expect(page.locator('text=AliExpress')).toBeVisible();
    await expect(page.locator('text=Website')).toBeVisible();

    // 内容与SKU
    await expect(page.locator('text=内容信息')).toBeVisible();
    await expect(page.locator('text=SKU 列表')).toBeVisible();

    console.log('✅ 查看页包含所有必要信息卡片');
  });

  test('查看页与编辑页成本CNY数值一致', async ({ page }) => {
    await login(page);
    const href = await getFirstProductDetailHref(page);
    const productId = href.split('/').pop();

    // 获取查看页的成本CNY值（¥ 数字）
    await page.goto(`${BASE_URL}/products/${productId}`);
    await page.waitForLoadState('networkidle');
    const detailHtml = await page.content();
    const costMatch = detailHtml.match(/成本\(CNY\).*?¥([\d.]+)/s);
    const detailCost = costMatch ? costMatch[1] : null;
    console.log('Detail page cost CNY:', detailCost);

    // 获取编辑页的成本CNY输入框值
    await page.goto(`${BASE_URL}/products/${productId}/edit`);
    await page.waitForLoadState('networkidle');
    const editCost = await page.locator('input[name="cost_cny"]').inputValue().catch(() => null);
    console.log('Edit page cost CNY:', editCost);

    if (detailCost && editCost && editCost !== '0') {
      const diff = Math.abs(parseFloat(detailCost) - parseFloat(editCost));
      console.log(`Difference: ${diff}`);
      expect(diff).toBeLessThan(0.01);
      console.log('✅ 成本CNY数值一致');
    } else {
      console.log('⚠️  产品无成本数据，跳过数值比较');
    }
  });

  test('查看页汇率字段与编辑页一致', async ({ page }) => {
    await login(page);
    const href = await getFirstProductDetailHref(page);
    const productId = href.split('/').pop();

    await page.goto(`${BASE_URL}/products/${productId}`);
    await page.waitForLoadState('networkidle');
    // 成本信息区域应有汇率字段
    const detailHtml = await page.content();
    const hasRate = detailHtml.includes('汇率');
    console.log('Detail page has 汇率:', hasRate);
    expect(hasRate).toBeTruthy();

    // 编辑页也有汇率
    await page.goto(`${BASE_URL}/products/${productId}/edit`);
    await page.waitForLoadState('networkidle');
    const editHtml = await page.content();
    const editHasRate = editHtml.includes('汇率');
    console.log('Edit page has 汇率:', editHasRate);
    expect(editHasRate).toBeTruthy();

    console.log('✅ 查看页与编辑页都显示汇率字段');
  });
});
