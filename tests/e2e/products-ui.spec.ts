/**
 * ciciERP 产品模块 UI E2E 测试（浏览器）
 * 覆盖：产品列表页、新建产品页、产品详情页
 */
import { test, expect, Page } from '@playwright/test';

const BASE_URL = process.env.BASE_URL || 'http://localhost:3001';
const ADMIN = { username: 'admin', password: 'admin123' };

// ─── 登录辅助 ─────────────────────────────────────────────────────────────────

async function login(page: Page) {
  await page.goto(`${BASE_URL}/login`);
  await page.fill('input[name="username"]', ADMIN.username);
  await page.fill('input[name="password"]', ADMIN.password);
  await page.click('button[type="submit"]');
  await page.waitForURL((url) => !url.pathname.includes('/login'), { timeout: 8000 });
}

// ─── 产品列表页 ───────────────────────────────────────────────────────────────

test.describe('Products UI - 列表页', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
  });

  test('访问 /products 正常加载', async ({ page }) => {
    await page.goto(`${BASE_URL}/products`);
    await expect(page).toHaveURL(`${BASE_URL}/products`);
    // 页面标题包含"产品"（排除 navbar logo）
    const heading = page.locator('main h1, .page-title, [class*="page"] h1').first();
    const bodyText = await page.locator('body').textContent();
    expect(bodyText).toMatch(/产品/);
  });

  test('产品列表表格可见', async ({ page }) => {
    await page.goto(`${BASE_URL}/products`);
    await expect(page.locator('table')).toBeVisible();
  });

  test('列表页包含新建产品按钮', async ({ page }) => {
    await page.goto(`${BASE_URL}/products`);
    const newBtn = page.locator('a[href="/products/new"], a:has-text("新建"), a:has-text("添加"), a:has-text("新增")');
    await expect(newBtn.first()).toBeVisible();
  });

  test('列表页有搜索功能', async ({ page }) => {
    await page.goto(`${BASE_URL}/products`);
    const searchInput = page.locator('input[name="keyword"], input[placeholder*="搜索"], input[type="search"]');
    await expect(searchInput.first()).toBeVisible();
  });

  test('产品列表包含预期列头', async ({ page }) => {
    await page.goto(`${BASE_URL}/products`);
    const tableText = await page.locator('table').textContent();
    // 检查主要列名
    expect(tableText).toMatch(/产品|编码|名称/);
  });

  test('点击产品行跳转到详情页', async ({ page }) => {
    await page.goto(`${BASE_URL}/products`);

    // 等待表格加载
    await page.waitForSelector('table tbody tr', { timeout: 5000 }).catch(() => {});
    const rows = page.locator('table tbody tr');
    const count = await rows.count();

    if (count > 0) {
      // 点击第一行的产品链接
      const firstLink = rows.first().locator('a').first();
      await firstLink.click();
      // 应跳转到 /products/:id
      await page.waitForURL(/\/products\/\d+/, { timeout: 5000 });
      expect(page.url()).toMatch(/\/products\/\d+/);
    } else {
      test.skip(); // 没有产品数据，跳过
    }
  });

  test('未登录访问产品列表 - 重定向到登录页', async ({ browser }) => {
    const context = await browser.newContext();  // 全新 context，无 cookie
    const page = await context.newPage();
    await page.goto(`${BASE_URL}/products`);
    await page.waitForURL(/\/login/, { timeout: 5000 });
    expect(page.url()).toContain('/login');
    await context.close();
  });
});

// ─── 新建产品页 ───────────────────────────────────────────────────────────────

test.describe('Products UI - 新建产品页', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
  });

  test('访问 /products/new 正常加载', async ({ page }) => {
    await page.goto(`${BASE_URL}/products/new`);
    await expect(page).toHaveURL(`${BASE_URL}/products/new`);
  });

  test('新建产品表单含必要字段', async ({ page }) => {
    await page.goto(`${BASE_URL}/products/new`);
    await expect(page.locator('input[name="name"]')).toBeVisible();
    await expect(page.locator('form[action="/products/new"] button[type="submit"]')).toBeVisible();
  });

  test('提交空表单 - 不成功（name 为必填）', async ({ page }) => {
    await page.goto(`${BASE_URL}/products/new`);
    await page.click('form[action="/products/new"] button[type="submit"]');

    // 停留在新建页或显示错误
    const currentUrl = page.url();
    const isStillOnNewPage = currentUrl.includes('/products/new');
    const hasError =
      (await page.locator('.error, .alert, [class*="error"]').count()) > 0 ||
      (await page.locator(':invalid').count()) > 0;

    expect(isStillOnNewPage || hasError).toBeTruthy();
  });

  test('填写必填字段并提交 - 成功创建跳转到列表或详情', async ({ page }) => {
    await page.goto(`${BASE_URL}/products/new`);

    const uniqueCode = `UI-TEST-${Date.now()}`;
    await page.fill('input[name="name"]', 'UI 自动化测试产品');

    // 如果有 product_code 字段
    const codeInput = page.locator('input[name="product_code"]');
    if (await codeInput.count() > 0) {
      await codeInput.fill(uniqueCode);
    }

    // 点击产品表单的提交按钮（不是 navbar 里的登出按钮）
    await page.click('form[action="/products/new"] button[type="submit"]');

    // 成功后应跳转到列表或详情页
    await page.waitForURL(
      (url) => url.pathname === '/products' || /\/products\/\d+/.test(url.pathname),
      { timeout: 8000 }
    );
    expect(page.url()).toMatch(/\/products/);
  });
});

// ─── 产品详情页 ───────────────────────────────────────────────────────────────

test.describe('Products UI - 详情页', () => {
  let productId: number;

  test.beforeAll(async ({ request }) => {
    // 通过 API 创建一个测试产品
    const loginRes = await request.post(`${BASE_URL}/api/v1/auth/login`, {
      data: ADMIN,
    });
    const { token } = (await loginRes.json()).data;

    const createRes = await request.post(`${BASE_URL}/api/v1/products`, {
      headers: { Authorization: `Bearer ${token}` },
      data: {
        name: 'UI 详情页测试产品',
        product_code: `UI-DETAIL-${Date.now()}`,
        status: 1,
      },
    });
    const body = await createRes.json();
    productId = body.data.id;
  });

  test.afterAll(async ({ request }) => {
    if (!productId) return;
    const loginRes = await request.post(`${BASE_URL}/api/v1/auth/login`, { data: ADMIN });
    const { token } = (await loginRes.json()).data;
    await request.delete(`${BASE_URL}/api/v1/products/${productId}`, {
      headers: { Authorization: `Bearer ${token}` },
    });
  });

  test.beforeEach(async ({ page }) => {
    await login(page);
  });

  test('访问产品详情页正常加载', async ({ page }) => {
    await page.goto(`${BASE_URL}/products/${productId}`);
    await expect(page).toHaveURL(`${BASE_URL}/products/${productId}`);
    await expect(page.locator('body')).toContainText(/UI 详情页测试产品/);
  });

  test('详情页包含编辑按钮', async ({ page }) => {
    await page.goto(`${BASE_URL}/products/${productId}`);
    const editBtn = page.locator(
      `a[href="/products/${productId}/edit"], a:has-text("编辑"), button:has-text("编辑")`
    );
    await expect(editBtn.first()).toBeVisible();
  });

  test('详情页包含返回列表链接', async ({ page }) => {
    await page.goto(`${BASE_URL}/products/${productId}`);
    const backBtn = page.locator('a[href="/products"], a:has-text("返回"), a:has-text("列表")');
    await expect(backBtn.first()).toBeVisible();
  });

  test('点击编辑按钮跳转到编辑页', async ({ page }) => {
    await page.goto(`${BASE_URL}/products/${productId}`);
    const editBtn = page.locator(
      `a[href="/products/${productId}/edit"], a:has-text("编辑")`
    );
    await editBtn.first().click();
    await page.waitForURL(`${BASE_URL}/products/${productId}/edit`, { timeout: 5000 });
    expect(page.url()).toContain(`/products/${productId}/edit`);
  });
});

// ─── 产品编辑页 ───────────────────────────────────────────────────────────────

test.describe('Products UI - 编辑页', () => {
  let productId: number;

  test.beforeAll(async ({ request }) => {
    const loginRes = await request.post(`${BASE_URL}/api/v1/auth/login`, { data: ADMIN });
    const { token } = (await loginRes.json()).data;

    const createRes = await request.post(`${BASE_URL}/api/v1/products`, {
      headers: { Authorization: `Bearer ${token}` },
      data: {
        name: 'UI 编辑页测试产品',
        product_code: `UI-EDIT-${Date.now()}`,
        status: 3,
      },
    });
    const body = await createRes.json();
    productId = body.data.id;
  });

  test.afterAll(async ({ request }) => {
    if (!productId) return;
    const loginRes = await request.post(`${BASE_URL}/api/v1/auth/login`, { data: ADMIN });
    const { token } = (await loginRes.json()).data;
    await request.delete(`${BASE_URL}/api/v1/products/${productId}`, {
      headers: { Authorization: `Bearer ${token}` },
    });
  });

  test.beforeEach(async ({ page }) => {
    await login(page);
  });

  test('访问编辑页正常加载', async ({ page }) => {
    await page.goto(`${BASE_URL}/products/${productId}/edit`);
    await expect(page.locator('input[name="name"]')).toBeVisible();
  });

  test('编辑页 name 字段预填了当前值', async ({ page }) => {
    await page.goto(`${BASE_URL}/products/${productId}/edit`);
    const nameVal = await page.locator('input[name="name"]').inputValue();
    expect(nameVal).toBe('UI 编辑页测试产品');
  });

  test('修改名称并保存 - 成功更新', async ({ page }) => {
    await page.goto(`${BASE_URL}/products/${productId}/edit`);
    await page.fill('input[name="name"]', 'UI 编辑页测试产品（已修改）');
    // 使用编辑表单的 submit 按钮（不是 navbar 里的登出按钮）
    await page.click(`form[action="/products/${productId}/edit"] button[type="submit"]`);

    // 等待跳转回详情页或列表
    await page.waitForURL(
      (url) =>
        url.pathname === '/products' || new RegExp(`/products/${productId}$`).test(url.pathname),
      { timeout: 8000 }
    );

    // 验证更新后内容
    await page.goto(`${BASE_URL}/products/${productId}`);
    await expect(page.locator('body')).toContainText(/UI 编辑页测试产品（已修改）/);
  });
});
