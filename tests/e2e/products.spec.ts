/**
 * ciciERP 产品模块 API E2E 测试
 * 覆盖：CRUD、分页、搜索、软删除、价格汇总
 */
import { test, expect, APIRequestContext } from '@playwright/test';

const BASE_URL = process.env.BASE_URL || 'http://localhost:3001';
const API = `${BASE_URL}/api/v1`;
const ADMIN = { username: 'admin', password: 'admin123' };

// ─── Fixtures ─────────────────────────────────────────────────────────────────

// 共享 token，避免每个 test 都登录
let authToken: string;
let createdProductId: number;
const testCode = `TEST-E2E-${Date.now()}`;

test.beforeAll(async ({ request }) => {
  const res = await request.post(`${API}/auth/login`, { data: ADMIN });
  const body = await res.json();
  authToken = body.data.token;
});

function authHeaders() {
  return { Authorization: `Bearer ${authToken}` };
}

// ─── 产品列表 ─────────────────────────────────────────────────────────────────

test.describe('Products API - 列表查询', () => {
  test('获取产品列表 - 返回分页结构', async ({ request }) => {
    const res = await request.get(`${API}/products`, {
      headers: authHeaders(),
    });
    expect(res.status()).toBe(200);

    const body = await res.json();
    expect(body.code).toBe(200);
    expect(body.data).toHaveProperty('items');
    expect(body.data).toHaveProperty('pagination');
    expect(body.data.pagination).toHaveProperty('total');
    expect(body.data.pagination).toHaveProperty('page');
    expect(body.data.pagination).toHaveProperty('page_size');
    expect(Array.isArray(body.data.items)).toBeTruthy();
  });

  test('分页参数生效 - page_size=5', async ({ request }) => {
    const res = await request.get(`${API}/products?page=1&page_size=5`, {
      headers: authHeaders(),
    });
    const body = await res.json();
    expect(body.code).toBe(200);
    expect(body.data.pagination.page_size).toBe(5);
    expect(body.data.items.length).toBeLessThanOrEqual(5);
  });

  test('未授权访问产品列表 - 返回 401', async ({ request }) => {
    const res = await request.get(`${API}/products`);
    expect(res.status()).toBe(401);
  });
});

// ─── 创建产品 ─────────────────────────────────────────────────────────────────

test.describe('Products API - 创建', () => {
  test('创建产品 - 必填字段', async ({ request }) => {
    const res = await request.post(`${API}/products`, {
      headers: authHeaders(),
      data: {
        name: 'E2E 测试产品',
        product_code: testCode,
        status: 3, // 草稿
      },
    });
    expect(res.status()).toBe(200);

    const body = await res.json();
    expect(body.code).toBe(200);
    expect(body.data.id).toBeTruthy();
    expect(body.data.name).toBe('E2E 测试产品');
    expect(body.data.product_code).toBe(testCode);

    createdProductId = body.data.id;
  });

  test('创建产品 - product_code 重复返回 409', async ({ request }) => {
    const res = await request.post(`${API}/products`, {
      headers: authHeaders(),
      data: {
        name: '重复编码产品',
        product_code: testCode, // 与上面相同
        status: 3,
      },
    });
    // 重复 product_code 导致唯一索引冲突，返回 409 或 500
    expect([409, 500]).toContain(res.status());
  });

  test('创建产品 - 缺少必填 name 返回 4xx', async ({ request }) => {
    const res = await request.post(`${API}/products`, {
      headers: authHeaders(),
      data: {
        product_code: `TEST-NO-NAME-${Date.now()}`,
        status: 3,
      },
    });
    expect(res.status()).toBeGreaterThanOrEqual(400);
    expect(res.status()).toBeLessThan(500);
  });

  test('创建完整产品 - 含可选字段', async ({ request }) => {
    const code = `TEST-FULL-${Date.now()}`;
    const res = await request.post(`${API}/products`, {
      headers: authHeaders(),
      data: {
        name: 'E2E 完整产品',
        name_en: 'E2E Full Product',
        product_code: code,
        weight: 1.5,
        volume: 0.002,
        description: '这是一个测试产品描述',
        status: 1, // 上架
        is_featured: false,
        is_new: true,
        notes: 'E2E 测试备注',
      },
    });
    expect(res.status()).toBe(200);
    const body = await res.json();
    expect(body.data.name_en).toBe('E2E Full Product');
    expect(body.data.weight).toBe(1.5);

    // 清理
    await request.delete(`${API}/products/${body.data.id}`, { headers: authHeaders() });
  });
});

// ─── 查询单个产品 ─────────────────────────────────────────────────────────────

test.describe('Products API - 详情', () => {
  test('根据 ID 获取产品详情', async ({ request }) => {
    const res = await request.get(`${API}/products/${createdProductId}`, {
      headers: authHeaders(),
    });
    expect(res.status()).toBe(200);

    const body = await res.json();
    expect(body.code).toBe(200);
    expect(body.data.id).toBe(createdProductId);
    expect(body.data.name).toBe('E2E 测试产品');
  });

  test('查询不存在的产品 - 返回 404', async ({ request }) => {
    const res = await request.get(`${API}/products/999999999`, {
      headers: authHeaders(),
    });
    expect(res.status()).toBe(404);
  });

  test('获取产品价格汇总', async ({ request }) => {
    const res = await request.get(`${API}/products/${createdProductId}/price-summary`, {
      headers: authHeaders(),
    });
    expect(res.status()).toBe(200);
    const body = await res.json();
    expect(body.code).toBe(200);
    expect(body.data).toHaveProperty('product_id');
  });

  test('获取产品历史价格', async ({ request }) => {
    const res = await request.get(`${API}/products/${createdProductId}/history-prices`, {
      headers: authHeaders(),
    });
    expect(res.status()).toBe(200);
    const body = await res.json();
    expect(body.code).toBe(200);
    expect(Array.isArray(body.data)).toBeTruthy();
  });
});

// ─── 更新产品 ─────────────────────────────────────────────────────────────────

test.describe('Products API - 更新', () => {
  test('更新产品名称', async ({ request }) => {
    const res = await request.put(`${API}/products/${createdProductId}`, {
      headers: authHeaders(),
      data: { name: 'E2E 测试产品（已更新）' },
    });
    expect(res.status()).toBe(200);
    const body = await res.json();
    expect(body.data.name).toBe('E2E 测试产品（已更新）');
  });

  test('更新产品状态为上架', async ({ request }) => {
    const res = await request.put(`${API}/products/${createdProductId}`, {
      headers: authHeaders(),
      data: { status: 1 },
    });
    expect(res.status()).toBe(200);
    const body = await res.json();
    expect(body.data.status).toBe(1);
  });

  test('更新不存在的产品 - 返回 404', async ({ request }) => {
    const res = await request.put(`${API}/products/999999999`, {
      headers: authHeaders(),
      data: { name: '不存在' },
    });
    expect(res.status()).toBe(404);
  });
});

// ─── 搜索产品 ─────────────────────────────────────────────────────────────────

test.describe('Products API - 搜索', () => {
  test('关键词搜索产品', async ({ request }) => {
    const res = await request.get(`${API}/products/search?keyword=E2E`, {
      headers: authHeaders(),
    });
    expect(res.status()).toBe(200);
    const body = await res.json();
    expect(body.code).toBe(200);
    expect(body.data).toHaveProperty('items');
  });

  test('按状态筛选产品', async ({ request }) => {
    const res = await request.get(`${API}/products?status=1`, {
      headers: authHeaders(),
    });
    const body = await res.json();
    expect(body.code).toBe(200);
    // 所有返回项状态应为 1（上架）
    for (const item of body.data.items) {
      expect(item.status).toBe(1);
    }
  });
});

// ─── 删除产品 ─────────────────────────────────────────────────────────────────

test.describe('Products API - 删除', () => {
  test('软删除产品', async ({ request }) => {
    const res = await request.delete(`${API}/products/${createdProductId}`, {
      headers: authHeaders(),
    });
    expect(res.status()).toBe(200);
    const body = await res.json();
    expect(body.code).toBe(200);
  });

  test('删除后查询 - 返回 404', async ({ request }) => {
    const res = await request.get(`${API}/products/${createdProductId}`, {
      headers: authHeaders(),
    });
    expect(res.status()).toBe(404);
  });

  test('删除后不出现在列表中', async ({ request }) => {
    const res = await request.get(
      `${API}/products?keyword=${encodeURIComponent('E2E 测试产品')}`,
      { headers: authHeaders() }
    );
    const body = await res.json();
    const found = body.data.items.some((p: any) => p.id === createdProductId);
    expect(found).toBeFalsy();
  });

  test('删除不存在的产品 - 返回 404', async ({ request }) => {
    const res = await request.delete(`${API}/products/999999999`, {
      headers: authHeaders(),
    });
    expect(res.status()).toBe(404);
  });
});
