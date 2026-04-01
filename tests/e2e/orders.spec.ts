/**
 * 订单模块 API 自动化测试
 *
 * 覆盖：列表、创建、详情、更新状态、发货、取消、PI/CI 下载
 */

import { test, expect } from '@playwright/test';

const API = 'http://localhost:3001/api/v1';
const ADMIN = { username: 'admin', password: 'admin123' };

let authToken: string;
let createdOrderId: number;
let cancelOrderId: number;

test.beforeAll(async ({ request }) => {
  const res = await request.post(`${API}/auth/login`, { data: ADMIN });
  const body = await res.json();
  authToken = body.data.token;
});

function authHeaders() {
  return { Authorization: `Bearer ${authToken}` };
}

// ─── 列表查询 ─────────────────────────────────────────────────────────────────

test.describe('Orders API - 列表查询', () => {
  test('获取订单列表 - 返回分页结构', async ({ request }) => {
    const res = await request.get(`${API}/orders`, { headers: authHeaders() });
    expect(res.status()).toBe(200);

    const body = await res.json();
    expect(body.code).toBe(200);
    expect(body.data).toHaveProperty('items');
    expect(body.data).toHaveProperty('pagination');
    expect(body.data.pagination).toHaveProperty('total');
    expect(Array.isArray(body.data.items)).toBeTruthy();
  });

  test('分页参数 page_size=5 生效', async ({ request }) => {
    const res = await request.get(`${API}/orders?page=1&page_size=5`, {
      headers: authHeaders(),
    });
    const body = await res.json();
    expect(body.code).toBe(200);
    expect(body.data.pagination.page_size).toBe(5);
    expect(body.data.items.length).toBeLessThanOrEqual(5);
  });

  test('按状态筛选 - order_status=1', async ({ request }) => {
    const res = await request.get(`${API}/orders?order_status=1`, {
      headers: authHeaders(),
    });
    const body = await res.json();
    expect(body.code).toBe(200);
    for (const order of body.data.items) {
      expect(order.order_status).toBe(1);
    }
  });

  test('未授权访问 - 返回 401', async ({ request }) => {
    const res = await request.get(`${API}/orders`);
    expect(res.status()).toBe(401);
  });
});

// ─── 创建订单 ─────────────────────────────────────────────────────────────────

test.describe('Orders API - 创建', () => {
  test('创建订单 - 仅必填字段', async ({ request }) => {
    const res = await request.post(`${API}/orders`, {
      headers: authHeaders(),
      data: {
        platform: 'manual',
        customer_name: 'E2E 测试客户',
        customer_mobile: '13800000001',
        items: [{ product_name: 'E2E 测试商品', quantity: 2, unit_price: 58.5 }],
        receiver_name: 'E2E 收件人',
        receiver_phone: '13800000001',
        country: 'China',
        address: 'E2E 测试地址 001',
      },
    });
    expect(res.status()).toBe(200);

    const body = await res.json();
    expect(body.code).toBe(200);
    expect(body.data.id).toBeTruthy();
    expect(body.data.order_status).toBe(1);
    expect(body.data.total_amount).toBeCloseTo(117.0, 1);

    createdOrderId = body.data.id;
  });

  test('创建订单 - 含运费和折扣', async ({ request }) => {
    const res = await request.post(`${API}/orders`, {
      headers: authHeaders(),
      data: {
        platform: 'manual',
        customer_name: 'E2E 运费测试',
        items: [{ product_name: '商品A', quantity: 1, unit_price: 100.0 }],
        receiver_name: '收件人',
        receiver_phone: '13800000002',
        country: 'US',
        address: 'Test Address US',
        shipping_fee: 20.0,
        discount_amount: 10.0,
      },
    });
    const body = await res.json();
    expect(body.code).toBe(200);
    // total = 100 + 20 - 10 = 110
    expect(body.data.total_amount).toBeCloseTo(110.0, 1);

    // 清理
    await request.post(`${API}/orders/${body.data.id}/cancel`, {
      headers: authHeaders(),
      data: { reason: 'test cleanup' },
    });
  });

  test('创建订单 - 无商品行返回错误', async ({ request }) => {
    const res = await request.post(`${API}/orders`, {
      headers: authHeaders(),
      data: {
        platform: 'manual',
        receiver_name: 'R',
        receiver_phone: '123',
        country: 'CN',
        address: 'A',
        items: [],
      },
    });
    expect(res.status()).toBeGreaterThanOrEqual(400);
  });

  test('创建取消测试用订单', async ({ request }) => {
    const res = await request.post(`${API}/orders`, {
      headers: authHeaders(),
      data: {
        platform: 'manual',
        customer_name: 'Cancel Test',
        items: [{ product_name: '待取消商品', quantity: 1, unit_price: 30.0 }],
        receiver_name: 'R',
        receiver_phone: '123',
        country: 'US',
        address: 'Addr',
      },
    });
    const body = await res.json();
    expect(body.code).toBe(200);
    cancelOrderId = body.data.id;
  });
});

// ─── 详情查询 ─────────────────────────────────────────────────────────────────

test.describe('Orders API - 详情', () => {
  test('根据 ID 获取订单详情', async ({ request }) => {
    const res = await request.get(`${API}/orders/${createdOrderId}`, {
      headers: authHeaders(),
    });
    expect(res.status()).toBe(200);

    const body = await res.json();
    expect(body.code).toBe(200);
    expect(body.data.id).toBe(createdOrderId);
    expect(body.data).toHaveProperty('items');
    expect(body.data.items.length).toBeGreaterThan(0);
    expect(body.data.items[0].product_name).toBe('E2E 测试商品');
  });

  test('查询不存在的订单 - 返回 404', async ({ request }) => {
    const res = await request.get(`${API}/orders/999999999`, {
      headers: authHeaders(),
    });
    expect(res.status()).toBe(404);
  });
});

// ─── 状态流转 ─────────────────────────────────────────────────────────────────

test.describe('Orders API - 状态流转', () => {
  test('状态 1→2 锁定价格', async ({ request }) => {
    const res = await request.post(`${API}/orders/${createdOrderId}/status`, {
      headers: authHeaders(),
      data: { status: 2 },
    });
    expect(res.status()).toBe(200);

    const detail = await request.get(`${API}/orders/${createdOrderId}`, {
      headers: authHeaders(),
    });
    const body = await detail.json();
    expect(body.data.order_status).toBe(2);
  });

  test('状态 2→3 已付款', async ({ request }) => {
    const res = await request.post(`${API}/orders/${createdOrderId}/status`, {
      headers: authHeaders(),
      data: { status: 3 },
    });
    expect(res.status()).toBe(200);
  });

  test('发货 - 创建物流记录', async ({ request }) => {
    const res = await request.post(`${API}/orders/${createdOrderId}/ship`, {
      headers: authHeaders(),
      data: {
        tracking_number: 'E2E-TRACK-001',
        logistics_name: 'DHL',
        shipping_note: 'E2E test shipment',
      },
    });
    expect(res.status()).toBe(200);
    const body = await res.json();
    expect(body.code).toBe(200);

    // 发货后状态应为 4
    const detail = await request.get(`${API}/orders/${createdOrderId}`, {
      headers: authHeaders(),
    });
    const detailBody = await detail.json();
    expect(detailBody.data.order_status).toBe(4);
  });

  test('取消订单 (status=1)', async ({ request }) => {
    const res = await request.post(`${API}/orders/${cancelOrderId}/cancel`, {
      headers: authHeaders(),
      data: { reason: 'E2E 测试取消' },
    });
    expect(res.status()).toBe(200);

    const detail = await request.get(`${API}/orders/${cancelOrderId}`, {
      headers: authHeaders(),
    });
    const body = await detail.json();
    expect(body.data.order_status).toBe(6);
    expect(body.data.cancel_reason).toBe('E2E 测试取消');
  });
});

// ─── 更新订单 ─────────────────────────────────────────────────────────────────

test.describe('Orders API - 更新', () => {
  test('更新内部备注', async ({ request }) => {
    // 用新订单测试更新
    const createRes = await request.post(`${API}/orders`, {
      headers: authHeaders(),
      data: {
        platform: 'manual',
        customer_name: 'Update Test',
        items: [{ product_name: 'Item', quantity: 1, unit_price: 10.0 }],
        receiver_name: 'R',
        receiver_phone: '123',
        country: 'US',
        address: 'Addr',
      },
    });
    const createBody = await createRes.json();
    const testId = createBody.data.id;

    const res = await request.put(`${API}/orders/${testId}`, {
      headers: authHeaders(),
      data: { internal_note: 'E2E internal note updated' },
    });
    expect(res.status()).toBe(200);

    const detail = await request.get(`${API}/orders/${testId}`, {
      headers: authHeaders(),
    });
    const body = await detail.json();
    expect(body.data.internal_note).toBe('E2E internal note updated');

    // 清理
    await request.post(`${API}/orders/${testId}/cancel`, {
      headers: authHeaders(),
      data: { reason: 'cleanup' },
    });
  });
});

// ─── PI / CI 下载 ─────────────────────────────────────────────────────────────

test.describe('Orders API - PI/CI 下载', () => {
  test('下载 PI - 状态 1 或 2 的订单', async ({ request }) => {
    // 找一个 status=1 的订单
    const listRes = await request.get(`${API}/orders?order_status=1&page_size=1`, {
      headers: authHeaders(),
    });
    const listBody = await listRes.json();
    const orderId = listBody.data.items[0]?.id;
    if (!orderId) return; // 没有可用订单则跳过

    const res = await request.get(`${API}/orders/${orderId}/download-pi`, {
      headers: authHeaders(),
    });
    expect(res.status()).toBe(200);
    // Excel 文件
    const contentType = res.headers()['content-type'] || '';
    expect(contentType).toMatch(/spreadsheet|excel|octet-stream/i);
  });

  test('下载 CI - 状态 3-5 的订单', async ({ request }) => {
    // 找一个 status=3 的订单
    const listRes = await request.get(`${API}/orders?order_status=3&page_size=1`, {
      headers: authHeaders(),
    });
    const listBody = await listRes.json();
    const orderId = listBody.data.items[0]?.id;
    if (!orderId) return;

    const res = await request.get(`${API}/orders/${orderId}/download-ci`, {
      headers: authHeaders(),
    });
    expect(res.status()).toBe(200);
  });
});
