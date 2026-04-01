/**
 * 订单模块浏览器 UI 自动化测试
 *
 * 覆盖：列表页、新建页（表单提交）、详情页、编辑页、状态操作按钮
 */

import { test, expect, Page } from '@playwright/test';

const BASE_URL = 'http://localhost:3001';
const ADMIN = { username: 'admin', password: 'admin123' };

async function login(page: Page) {
  await page.goto(`${BASE_URL}/login`);
  await page.fill('input[name="username"]', ADMIN.username);
  await page.fill('input[name="password"]', ADMIN.password);
  await page.click('button[type="submit"]');
  await page.waitForURL((url) => !url.pathname.includes('/login'), { timeout: 8000 });
}

// ─── 订单列表页 ───────────────────────────────────────────────────────────────

test.describe('Orders UI - 列表页', () => {
  test.beforeEach(async ({ page }) => { await login(page); });

  test('访问 /orders 显示订单列表', async ({ page }) => {
    await page.goto(`${BASE_URL}/orders`);
    await expect(page.locator('body')).toContainText(/订单/);
    // 有 table 或 list 内容
    const hasTable = (await page.locator('table').count()) > 0;
    const hasList  = (await page.locator('[class*="order"]').count()) > 0;
    expect(hasTable || hasList).toBeTruthy();
  });

  test('列表包含订单编号', async ({ page }) => {
    await page.goto(`${BASE_URL}/orders`);
    await expect(page.locator('body')).toContainText(/ORD/);
  });

  test('点击新建订单链接跳转到 /orders/new', async ({ page }) => {
    await page.goto(`${BASE_URL}/orders`);
    const newLink = page.locator('a[href="/orders/new"]').first();
    if (await newLink.count() > 0) {
      await newLink.click();
      await expect(page).toHaveURL(/\/orders\/new/);
    }
  });

  test('状态筛选链接存在', async ({ page }) => {
    await page.goto(`${BASE_URL}/orders`);
    // 筛选器是 <a href="/orders?status=N"> 链接形式
    const filterLinks = page.locator('a[href*="/orders?status="]');
    expect(await filterLinks.count()).toBeGreaterThan(0);
  });
});

// ─── 新建订单页 ───────────────────────────────────────────────────────────────

test.describe('Orders UI - 新建订单', () => {
  test.beforeEach(async ({ page }) => { await login(page); });

  test('新建页面包含必要表单字段', async ({ page }) => {
    await page.goto(`${BASE_URL}/orders/new`);
    await expect(page.locator('input[name="receiver_name"]')).toBeVisible();
    await expect(page.locator('input[name="receiver_phone"]')).toBeVisible();
    await expect(page.locator('input[name="country"]')).toBeVisible();
    await expect(page.locator('input[name="address"]')).toBeVisible();
  });

  test('包含商品选择区域', async ({ page }) => {
    await page.goto(`${BASE_URL}/orders/new`);
    // item_product[] 下拉列表
    await expect(page.locator('select[name="item_product[]"]').first()).toBeVisible();
    await expect(page.locator('input[name="item_quantity[]"]').first()).toBeVisible();
    await expect(page.locator('input[name="item_price[]"]').first()).toBeVisible();
  });

  test('填写表单并提交 - 成功创建后跳转到详情页', async ({ page }) => {
    await page.goto(`${BASE_URL}/orders/new`);

    // 填写收货信息（必填）
    await page.fill('input[name="receiver_name"]', 'UI 测试收件人');
    await page.fill('input[name="receiver_phone"]', '13900139999');
    await page.fill('input[name="country"]', 'China');
    await page.fill('input[name="address"]', 'UI 测试地址 888');

    // 填写客户名
    const customerNameInput = page.locator('input[name="customer_name"]');
    if (await customerNameInput.count() > 0) {
      await customerNameInput.fill('UI 测试客户');
    }

    // 选择商品（第一行，选择第二个有效选项）
    const productSelect = page.locator('select[name="item_product[]"]').first();
    const options = await productSelect.locator('option').all();
    // 找第一个有真实 value 的选项（跳过空值 "--")
    for (const opt of options) {
      const val = await opt.getAttribute('value');
      if (val && val !== '') {
        await productSelect.selectOption(val);
        break;
      }
    }

    // 设置数量和价格
    await page.fill('input[name="item_quantity[]"]', '1');
    await page.fill('input[name="item_price[]"]', '99.00');

    // 点击订单表单的提交按钮（不是 navbar 的登出按钮）
    await page.click('form[action="/orders/new"] button[type="submit"]');

    // 成功后跳转到订单详情页
    await page.waitForURL(
      (url) => /\/orders\/\d+$/.test(url.pathname),
      { timeout: 10000 }
    );
    expect(page.url()).toMatch(/\/orders\/\d+$/);
    await expect(page.locator('body')).toContainText(/ORD/);
  });
});

// ─── 订单详情页 ───────────────────────────────────────────────────────────────

test.describe('Orders UI - 详情页', () => {
  test.beforeEach(async ({ page }) => { await login(page); });

  test('访问已有订单详情 - 显示基本信息', async ({ page }) => {
    await page.goto(`${BASE_URL}/orders/1`);
    // 可能 404（已取消或不存在），尝试订单 2
    const status = await page.evaluate(() => document.body.innerText);
    if (status.includes('不存在') || status.includes('404')) {
      await page.goto(`${BASE_URL}/orders/2`);
    }
    await expect(page.locator('body')).toContainText(/ORD/);
  });

  test('详情页显示商品明细', async ({ page }) => {
    // 找一个有效的订单 ID（从列表取）
    const listRes = await page.request.get(`${BASE_URL}/api/v1/orders?page_size=1`, {
      headers: {
        Authorization: `Bearer ${await page.evaluate(() => {
          const cookies = document.cookie.split(';');
          return cookies.find((c) => c.trim().startsWith('auth_token='))?.split('=')[1] || '';
        })}`,
      },
    });

    await page.goto(`${BASE_URL}/orders`);
    // 点击第一条订单链接
    const firstOrderLink = page.locator('a[href*="/orders/"]').first();
    if (await firstOrderLink.count() > 0) {
      const href = await firstOrderLink.getAttribute('href');
      if (href && /\/orders\/\d+$/.test(href)) {
        await page.goto(`${BASE_URL}${href}`);
        await expect(page.locator('body')).toContainText(/ORD/);
      }
    }
  });

  test('status=1 订单显示编辑入口', async ({ page }) => {
    // 找 status=1 订单
    await page.goto(`${BASE_URL}/orders?order_status=1`);
    const firstLink = page.locator('a[href*="/orders/"]').first();
    if (await firstLink.count() > 0) {
      const href = await firstLink.getAttribute('href');
      if (href && /\/orders\/\d+$/.test(href)) {
        await page.goto(`${BASE_URL}${href}`);
        // 状态 1 应该有编辑或锁定按钮
        const hasEdit = (await page.locator('a[href*="/edit"]').count()) > 0;
        const hasAction = (await page.locator('button, a').filter({ hasText: /编辑|锁定|取消/ }).count()) > 0;
        expect(hasEdit || hasAction).toBeTruthy();
      }
    }
  });
});

// ─── 编辑订单页 ───────────────────────────────────────────────────────────────

test.describe('Orders UI - 编辑页', () => {
  let editOrderId: number;

  test.beforeAll(async ({ request }) => {
    // 创建一个 status=1 的订单供编辑
    const loginRes = await request.post(`${BASE_URL}/api/v1/auth/login`, {
      data: ADMIN,
    });
    const loginBody = await loginRes.json();
    const token = loginBody.data.token;

    const createRes = await request.post(`${BASE_URL}/api/v1/orders`, {
      headers: { Authorization: `Bearer ${token}` },
      data: {
        platform: 'manual',
        customer_name: 'UI Edit Test',
        items: [{ product_name: 'Edit Test Item', quantity: 1, unit_price: 50.0 }],
        receiver_name: 'Edit Receiver',
        receiver_phone: '13800000099',
        country: 'US',
        address: 'Edit Test Address',
      },
    });
    const createBody = await createRes.json();
    editOrderId = createBody.data.id;
  });

  test.beforeEach(async ({ page }) => { await login(page); });

  test('编辑页加载 - 表单含现有数据', async ({ page }) => {
    await page.goto(`${BASE_URL}/orders/${editOrderId}/edit`);
    // 收件人应该预填
    const receiverInput = page.locator('input[name="receiver_name"]');
    await expect(receiverInput).toBeVisible();
    const val = await receiverInput.inputValue();
    expect(val).toBe('Edit Receiver');
  });

  test('修改收件人并保存', async ({ page }) => {
    await page.goto(`${BASE_URL}/orders/${editOrderId}/edit`);
    await page.fill('input[name="receiver_name"]', 'UI 修改后收件人');

    await page.click(`form[action="/orders/${editOrderId}/edit"] button[type="submit"]`);

    // 保存后跳转回详情页
    await page.waitForURL(
      (url) => new RegExp(`/orders/${editOrderId}$`).test(url.pathname) || url.pathname === '/orders',
      { timeout: 8000 }
    );

    // 重新打开详情确认更新
    await page.goto(`${BASE_URL}/orders/${editOrderId}`);
    await expect(page.locator('body')).toContainText(/UI 修改后收件人/);
  });
});

// ─── 状态操作按钮 ─────────────────────────────────────────────────────────────

test.describe('Orders UI - 状态操作', () => {
  test.beforeEach(async ({ page }) => { await login(page); });

  test('status=1 订单详情页有锁定/取消按钮', async ({ page }) => {
    await page.goto(`${BASE_URL}/orders?order_status=1`);
    const firstLink = page.locator('a[href*="/orders/"]').first();
    if (await firstLink.count() === 0) return; // 没有符合条件订单则跳过

    const href = await firstLink.getAttribute('href');
    if (!href || !/\/orders\/\d+$/.test(href)) return;

    await page.goto(`${BASE_URL}${href}`);
    const actionBtn = page.locator('button, a').filter({ hasText: /锁定|取消|Confirm|Cancel/ });
    expect(await actionBtn.count()).toBeGreaterThan(0);
  });

  test('PI 下载链接在 status≤2 订单可见', async ({ page }) => {
    await page.goto(`${BASE_URL}/orders?order_status=1`);
    const firstLink = page.locator('a[href*="/orders/"]').first();
    if (await firstLink.count() === 0) return;

    const href = await firstLink.getAttribute('href');
    if (!href || !/\/orders\/\d+$/.test(href)) return;

    await page.goto(`${BASE_URL}${href}`);
    const piLink = page.locator('a[href*="download-pi"]');
    expect(await piLink.count()).toBeGreaterThan(0);
  });

  test('CI 下载链接在 status≥3 订单可见', async ({ page }) => {
    await page.goto(`${BASE_URL}/orders?order_status=3`);
    const firstLink = page.locator('a[href*="/orders/"]').first();
    if (await firstLink.count() === 0) return;

    const href = await firstLink.getAttribute('href');
    if (!href || !/\/orders\/\d+$/.test(href)) return;

    await page.goto(`${BASE_URL}${href}`);
    const ciLink = page.locator('a[href*="download-ci"]');
    expect(await ciLink.count()).toBeGreaterThan(0);
  });
});
