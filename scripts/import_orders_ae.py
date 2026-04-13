#!/usr/bin/env python3
"""导入 orders_ae.xlsx 数据到 ciciERP 数据库"""

import sys
import json
import pandas as pd
import sqlite3
import random
import string
from datetime import datetime

# 配置
EXCEL_PATH = sys.argv[1] if len(sys.argv) > 1 else '/home/wxy/.xiaozhi/files/file_1774829048324_a84af3a2_orders_ae.xlsx.xlsx'
DB_PATH = '/home/wxy/data/ciciERP/data/cicierp.db'

def generate_customer_code():
    """生成客户编码: CUS-YYYYMMDD-XXXX"""
    date_str = datetime.now().strftime('%Y%m%d')
    rand_str = ''.join(random.choices(string.ascii_uppercase + string.digits, k=4))
    return f"CUS-{date_str}-{rand_str}"

def main():
    # 读取 Excel
    df = pd.read_excel(EXCEL_PATH)
    print(f"读取到 {len(df)} 条订单记录")

    # 连接数据库
    conn = sqlite3.connect(DB_PATH)
    cursor = conn.cursor()

    # 统计
    stats = {
        'products_added': 0,
        'customers_added': 0,
        'orders_added': 0,
        'items_added': 0,
        'orders_skipped': 0,
    }

    # ========== 1. 处理产品 ==========
    print("\n=== 处理产品 ===")

    # Excel 中的所有产品
    excel_products = df['Product'].dropna().unique()
    print(f"Excel 中的产品: {list(excel_products)}")

    # 查询现有产品（按名称匹配）
    cursor.execute('SELECT id, name FROM products WHERE deleted_at IS NULL')
    existing_products = {row[1]: row[0] for row in cursor.fetchall()}

    # 产品映射：自动创建不存在的产品
    product_map = {}
    for prod_name in excel_products:
        prod_name = str(prod_name).strip()
        if not prod_name:
            continue
        if prod_name in existing_products:
            product_map[prod_name] = existing_products[prod_name]
            print(f"  产品已存在: {prod_name} (ID: {product_map[prod_name]})")
        else:
            # 自动创建占位产品，后续可在产品页完善
            cursor.execute('''
                INSERT INTO products (name, purchase_price, sale_price, status)
                VALUES (?, 0, 0, 1)
            ''', (prod_name,))
            product_map[prod_name] = cursor.lastrowid
            stats['products_added'] += 1
            print(f"  新增产品: {prod_name} (ID: {product_map[prod_name]})")

    conn.commit()

    # ========== 2. 处理客户 ==========
    print("\n=== 处理客户 ===")

    customer_names = df['Client Name'].unique()
    print(f"Excel 中的客户数: {len(customer_names)}")

    # 重新查询现有客户
    cursor.execute('SELECT id, customer_code, name FROM customers')
    existing_customers = {row[2]: {'id': row[0], 'code': row[1]} for row in cursor.fetchall()}

    customer_map = {}

    for name in customer_names:
        if pd.isna(name) or not str(name).strip():
            continue
        name = str(name).strip()
        if name in existing_customers:
            customer_map[name] = existing_customers[name]['id']
        else:
            code = generate_customer_code()
            cursor.execute('''
                INSERT INTO customers (customer_code, name, source)
                VALUES (?, ?, 'import')
            ''', (code, name))
            customer_map[name] = cursor.lastrowid
            stats['customers_added'] += 1

    conn.commit()
    print(f"  新增客户: {stats['customers_added']} 个")

    # ========== 3. 处理订单 ==========
    print("\n=== 处理订单 ===")

    # 重新查询现有订单号
    cursor.execute('SELECT order_code FROM orders')
    existing_order_codes = set(row[0] for row in cursor.fetchall())
    print(f"数据库中已有订单: {len(existing_order_codes)} 个")

    for _, row in df.iterrows():
        order_code = str(row['Order No.']).strip()
        customer_name = str(row['Client Name']).strip() if pd.notna(row['Client Name']) else ''
        product_name = str(row['Product']).strip() if pd.notna(row['Product']) else ''

        # 跳过已存在的订单
        if order_code in existing_order_codes:
            stats['orders_skipped'] += 1
            continue

        # 跳过没有产品映射的订单
        if product_name not in product_map:
            print(f"  跳过订单 {order_code}: 产品 '{product_name}' 不存在")
            stats['orders_skipped'] += 1
            continue

        # 跳过没有客户的订单
        if customer_name not in customer_map:
            print(f"  跳过订单 {order_code}: 客户 '{customer_name}' 不存在")
            stats['orders_skipped'] += 1
            continue

        try:
            # 解析日期
            order_date = row['Date']
            if pd.notna(order_date):
                if isinstance(order_date, str):
                    created_at = order_date
                else:
                    created_at = pd.to_datetime(order_date).strftime('%Y-%m-%d %H:%M:%S')
            else:
                created_at = datetime.now().strftime('%Y-%m-%d %H:%M:%S')

            # 解析金额
            qty = int(row['Qty']) if pd.notna(row['Qty']) else 1
            unit_price = float(row['Sales Unit Price (RMB)']) if pd.notna(row['Sales Unit Price (RMB)']) else 0
            total_amount = float(row['Order Amount (RMB)']) if pd.notna(row['Order Amount (RMB)']) else 0
            cost_price = float(row['Cost per Unit (RMB)']) if pd.notna(row['Cost per Unit (RMB)']) else 0

            # 创建订单
            cursor.execute('''
                INSERT INTO orders (
                    order_code, platform, customer_id, customer_name,
                    order_status, payment_status, fulfillment_status,
                    total_amount, subtotal, paid_amount, currency,
                    created_at, updated_at
                ) VALUES (?, 'aliexpress', ?, ?, 5, 3, 3, ?, ?, ?, 'RMB', ?, ?)
            ''', (
                order_code,
                customer_map[customer_name],
                customer_name,
                total_amount,
                total_amount,
                total_amount,
                created_at,
                created_at
            ))
            order_id = cursor.lastrowid
            stats['orders_added'] += 1

            # 创建订单项
            cursor.execute('''
                INSERT INTO order_items (
                    order_id, product_id, product_name, product_code,
                    quantity, unit_price, subtotal, total_amount, cost_price
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            ''', (
                order_id,
                product_map[product_name],
                product_name,
                None,
                qty,
                unit_price,
                total_amount,
                total_amount,
                cost_price
            ))
            stats['items_added'] += 1

            # 添加到已存在集合，避免重复
            existing_order_codes.add(order_code)

        except Exception as e:
            print(f"  订单 {order_code} 导入失败: {e}")
            stats['orders_skipped'] += 1
            continue

    conn.commit()
    conn.close()

    # ========== 统计信息 ==========
    print("\n" + "="*50)
    print("导入完成统计")
    print("="*50)
    print(f"产品: 新增 {stats['products_added']} 个")
    print(f"客户: 新增 {stats['customers_added']} 个")
    print(f"订单: 新增 {stats['orders_added']} 条")
    print(f"订单项: 新增 {stats['items_added']} 条")
    print(f"跳过: {stats['orders_skipped']} 条")
    print("\n✅ 导入完成")
    print("\n__JSON_SUMMARY__")
    print(json.dumps({
        "orders_added": stats['orders_added'],
        "customers_added": stats['customers_added'],
        "skipped": stats['orders_skipped'],
    }))

if __name__ == '__main__':
    main()
