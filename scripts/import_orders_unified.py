#!/usr/bin/env python3
"""
统一订单导入脚本 — 支持 Alibaba (USD) 和 AliExpress (RMB) 混合导入
列顺序：日期 | 订单号* | 客户姓名 | 产品名称* | 数量* | 平台*(alibaba/aliexpress) | 单价* | 币种(USD/RMB) | 成本 | 备注
"""

import sys
import json
import random
import string
import sqlite3
import pandas as pd
from datetime import datetime

EXCEL_PATH = sys.argv[1] if len(sys.argv) > 1 else 'orders_import_template.xlsx'
DB_PATH = sys.argv[2] if len(sys.argv) > 2 else '/home/wxy/data/ciciERP/data/cicierp.db'
EXCHANGE_RATE = 7.2  # 默认汇率，若数据库有更新值则使用最新值


def generate_customer_code():
    date_str = datetime.now().strftime('%Y%m%d')
    rand_str = ''.join(random.choices(string.ascii_uppercase + string.digits, k=4))
    return f"CUS-{date_str}-{rand_str}"


def generate_order_code():
    date_str = datetime.now().strftime('%Y%m%d')
    rand_str = ''.join(random.choices(string.ascii_uppercase + string.digits, k=6))
    return f"ORD-{date_str}-{rand_str}"


def main():
    print(f"=== 开始导入统一订单数据 ===")
    print(f"文件: {EXCEL_PATH}")
    print(f"数据库: {DB_PATH}")

    df = pd.read_excel(EXCEL_PATH)
    # 统一列名（模板列：日期|订单号|客户姓名|产品名称|数量|平台|单价|币种|成本|备注）
    df.columns = [str(c).strip() for c in df.columns]
    # 过滤示例行（订单号以 ORD-20250814 开头的等示例）
    df = df.dropna(subset=[df.columns[1]])  # 订单号不能为空

    print(f"读取到 {len(df)} 行数据")

    conn = sqlite3.connect(DB_PATH)
    cursor = conn.cursor()

    # 获取当前汇率
    cursor.execute("SELECT rate FROM exchange_rates WHERE from_currency='USD' AND to_currency='CNY' ORDER BY effective_date DESC LIMIT 1")
    rate_row = cursor.fetchone()
    exchange_rate = float(rate_row[0]) if rate_row else EXCHANGE_RATE
    print(f"使用汇率: {exchange_rate}")

    # 客户/产品映射
    cursor.execute('SELECT id, name FROM customers WHERE deleted_at IS NULL')
    customers_map = {r[1].strip().lower(): r[0] for r in cursor.fetchall()}

    cursor.execute('SELECT id, name FROM products WHERE deleted_at IS NULL')
    products_map = {r[1].strip().lower(): r[0] for r in cursor.fetchall()}

    cursor.execute('SELECT order_code FROM orders')
    existing_orders = {r[0] for r in cursor.fetchall()}

    stats = {
        'orders_added': 0,
        'customers_added': 0,
        'skipped': 0,
        'errors': [],
    }

    # 按订单号分组
    order_col = df.columns[1]
    grouped = df.groupby(order_col)

    for order_no_raw, group in grouped:
        order_no = str(order_no_raw).strip()
        if not order_no:
            continue

        if order_no in existing_orders:
            stats['skipped'] += 1
            print(f"  跳过已存在订单: {order_no}")
            continue

        first_row = group.iloc[0]
        cols = df.columns

        # 解析日期
        raw_date = str(first_row[cols[0]]).strip() if pd.notna(first_row[cols[0]]) else ''
        try:
            order_date = pd.to_datetime(raw_date).strftime('%Y-%m-%d') if raw_date else datetime.now().strftime('%Y-%m-%d')
        except Exception:
            order_date = datetime.now().strftime('%Y-%m-%d')

        customer_name = str(first_row[cols[2]]).strip() if len(cols) > 2 and pd.notna(first_row[cols[2]]) else '未知客户'
        platform = str(first_row[cols[5]]).strip().lower() if len(cols) > 5 and pd.notna(first_row[cols[5]]) else 'alibaba'
        currency = str(first_row[cols[7]]).strip().upper() if len(cols) > 7 and pd.notna(first_row[cols[7]]) else ('USD' if platform == 'alibaba' else 'RMB')
        notes = str(first_row[cols[9]]).strip() if len(cols) > 9 and pd.notna(first_row[cols[9]]) else ''
        if notes == 'nan':
            notes = ''

        # 平台映射
        if platform not in ('alibaba', 'aliexpress'):
            platform = 'alibaba' if currency == 'USD' else 'aliexpress'

        # 客户处理
        customer_key = customer_name.strip().lower()
        customer_id = customers_map.get(customer_key)
        if not customer_id and customer_name and customer_name != '未知客户':
            code = generate_customer_code()
            cursor.execute(
                "INSERT INTO customers (customer_code, name, status, created_at, updated_at) VALUES (?, ?, 1, datetime('now'), datetime('now'))",
                (code, customer_name)
            )
            customer_id = cursor.lastrowid
            customers_map[customer_key] = customer_id
            stats['customers_added'] += 1
            print(f"  新增客户: {customer_name}")

        # 计算订单总金额
        order_amount_cny = 0.0
        order_amount_usd = 0.0
        items_data = []

        for _, row in group.iterrows():
            product_name = str(row[cols[3]]).strip() if len(cols) > 3 and pd.notna(row[cols[3]]) else ''
            if not product_name or product_name == 'nan':
                continue

            try:
                qty = int(float(str(row[cols[4]]).replace(',', ''))) if len(cols) > 4 and pd.notna(row[cols[4]]) else 1
            except (ValueError, TypeError):
                qty = 1

            try:
                unit_price = float(str(row[cols[6]]).replace(',', '')) if len(cols) > 6 and pd.notna(row[cols[6]]) else 0.0
            except (ValueError, TypeError):
                unit_price = 0.0

            try:
                cost = float(str(row[cols[8]]).replace(',', '')) if len(cols) > 8 and pd.notna(row[cols[8]]) else 0.0
            except (ValueError, TypeError):
                cost = 0.0

            # 统一换算
            if platform == 'alibaba' or currency == 'USD':
                unit_price_usd = unit_price
                unit_price_cny = unit_price * exchange_rate
                cost_usd = cost
            else:  # aliexpress, RMB
                unit_price_cny = unit_price
                unit_price_usd = unit_price / exchange_rate
                cost_usd = cost / exchange_rate

            order_amount_cny += unit_price_cny * qty
            order_amount_usd += unit_price_usd * qty

            # 查找产品
            product_id = products_map.get(product_name.strip().lower())

            items_data.append({
                'product_name': product_name,
                'product_id': product_id,
                'qty': qty,
                'unit_price_cny': unit_price_cny,
                'unit_price_usd': unit_price_usd,
                'cost_per_unit': cost_usd,
            })

        if not items_data:
            stats['skipped'] += 1
            continue

        # 插入订单
        final_currency = 'USD' if platform == 'alibaba' else 'CNY'
        final_amount = order_amount_usd if platform == 'alibaba' else order_amount_cny

        cursor.execute("""
            INSERT INTO orders (order_code, customer_id, platform, currency, order_amount,
                                order_date, status, notes, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, 'pending', ?, datetime('now'), datetime('now'))
        """, (order_no, customer_id, platform, final_currency, round(final_amount, 2),
              order_date, notes))
        order_id = cursor.lastrowid
        existing_orders.add(order_no)

        # 插入订单明细
        for item in items_data:
            unit_price = item['unit_price_usd'] if platform == 'alibaba' else item['unit_price_cny']
            cursor.execute("""
                INSERT INTO order_items (order_id, product_id, product_name, quantity,
                                        unit_price, cost_per_unit, currency, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, datetime('now'), datetime('now'))
            """, (order_id, item['product_id'], item['product_name'], item['qty'],
                  round(unit_price, 4), round(item['cost_per_unit'], 4), final_currency))

        stats['orders_added'] += 1
        print(f"  导入订单: {order_no} ({platform}, {len(items_data)} 行)")

    conn.commit()
    conn.close()

    print(f"\n=== 导入完成 ===")
    print(f"新增订单: {stats['orders_added']}")
    print(f"新增客户: {stats['customers_added']}")
    print(f"跳过记录: {stats['skipped']}")
    # 最后一行输出 JSON 摘要（Rust 端解析用）
    print(json.dumps({
        'orders_added': stats['orders_added'],
        'customers_added': stats['customers_added'],
        'skipped': stats['skipped'],
    }))


if __name__ == '__main__':
    main()
