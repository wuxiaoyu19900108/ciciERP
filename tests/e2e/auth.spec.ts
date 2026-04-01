/**
 * ciciERP 认证模块 E2E 测试
 * 覆盖：登录/登出、Token 验证、权限控制
 */
import { test, expect, APIRequestContext } from '@playwright/test';

const BASE_URL = process.env.BASE_URL || 'http://localhost:3001';
const API = `${BASE_URL}/api/v1`;

// 测试账号（与 migrations/001_init.sql 一致）
const ADMIN = { username: 'admin', password: 'admin123' };

// ─── API 认证测试 ─────────────────────────────────────────────────────────────

test.describe('Auth API', () => {
  test('登录成功 - 返回 token', async ({ request }) => {
    const res = await request.post(`${API}/auth/login`, { data: ADMIN });
    expect(res.status()).toBe(200);

    const body = await res.json();
    expect(body.code).toBe(200);
    expect(body.data.token).toBeTruthy();
    expect(body.data.token_type).toBe('Bearer');
    expect(body.data.user.username).toBe(ADMIN.username);
  });

  test('登录失败 - 密码错误', async ({ request }) => {
    const res = await request.post(`${API}/auth/login`, {
      data: { username: 'admin', password: 'wrongpassword' },
    });
    // 服务端密码校验失败返回 400
    expect(res.status()).toBe(400);
  });

  test('登录失败 - 用户名不存在', async ({ request }) => {
    const res = await request.post(`${API}/auth/login`, {
      data: { username: 'nonexistent_user_xyz', password: 'admin123' },
    });
    expect([400, 401]).toContain(res.status());
  });

  test('未登录访问受保护接口 - 返回 401', async ({ request }) => {
    const res = await request.get(`${API}/products`);
    expect(res.status()).toBe(401);
  });

  test('登录后获取当前用户信息 /auth/me', async ({ request }) => {
    // 先登录
    const loginRes = await request.post(`${API}/auth/login`, { data: ADMIN });
    const { token } = (await loginRes.json()).data;

    const res = await request.get(`${API}/auth/me`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    expect(res.status()).toBe(200);
    const body = await res.json();
    expect(body.data.username).toBe(ADMIN.username);
  });

  test('登出成功', async ({ request }) => {
    const loginRes = await request.post(`${API}/auth/login`, { data: ADMIN });
    const { token } = (await loginRes.json()).data;

    const res = await request.post(`${API}/auth/logout`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    expect(res.status()).toBe(200);
  });

  test('Token 过期或无效 - 返回 401', async ({ request }) => {
    const res = await request.get(`${API}/products`, {
      headers: { Authorization: 'Bearer invalid.token.here' },
    });
    expect(res.status()).toBe(401);
  });});

// ─── 登录页 UI 测试 ──────────────────────────────────────────────────────────

test.describe('Login Page UI', () => {
  test('登录页正常渲染', async ({ page }) => {
    await page.goto(`${BASE_URL}/login`);
    await expect(page).toHaveTitle(/ciciERP|登录/i);
    await expect(page.locator('input[name="username"]')).toBeVisible();
    await expect(page.locator('input[name="password"]')).toBeVisible();
    await expect(page.locator('button[type="submit"]')).toBeVisible();
  });

  test('输入错误密码 - 显示错误提示', async ({ page }) => {
    await page.goto(`${BASE_URL}/login`);
    await page.fill('input[name="username"]', 'admin');
    await page.fill('input[name="password"]', 'wrongpassword');
    await page.click('button[type="submit"]');

    // 等待页面响应后应显示错误信息
    await page.waitForURL(`${BASE_URL}/login**`);
    const errorVisible =
      (await page.locator('.error, .alert, [class*="error"], [class*="alert"]').count()) > 0 ||
      (await page.locator('text=/错误|失败|incorrect|invalid/i').count()) > 0;
    expect(errorVisible).toBeTruthy();
  });

  test('使用正确账号登录 - 跳转到首页', async ({ page }) => {
    await page.goto(`${BASE_URL}/login`);
    await page.fill('input[name="username"]', ADMIN.username);
    await page.fill('input[name="password"]', ADMIN.password);
    await page.click('button[type="submit"]');

    // 成功后应跳转离开 /login
    await page.waitForURL((url) => !url.pathname.includes('/login'), { timeout: 5000 });
    expect(page.url()).not.toContain('/login');
  });

  test('已登录用户访问受保护页面不跳转到登录页', async ({ page }) => {
    // 先登录
    await page.goto(`${BASE_URL}/login`);
    await page.fill('input[name="username"]', ADMIN.username);
    await page.fill('input[name="password"]', ADMIN.password);
    await page.click('button[type="submit"]');
    await page.waitForURL((url) => !url.pathname.includes('/login'), { timeout: 5000 });

    // 访问产品页应正常展示，不跳转回登录
    await page.goto(`${BASE_URL}/products`);
    expect(page.url()).not.toContain('/login');
    await expect(page.locator('body')).toBeVisible();
  });
});
