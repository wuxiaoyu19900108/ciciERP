#!/usr/bin/env python3
"""
导入阿里订单数据到 ciciERP 数据库
"""

import pandas as pd
import sqlite3
from datetime import datetime
import sys

# 配置
EXCEL_PATH = '/home/wxy/.xiaozhi/files/file_1774771486554_909bcff7_orders_ali.xlsx.xlsx'
DB_PATH = 'data/cicierp.db'
EXCHANGE_RATE = 7.2

# 产品名映射（Excel 名称 -> 数据库精确匹配）
PRODUCT_ALIASES = {
    'comfast-cf-ew71': 'COMFAST-CF-EW71',
    'comfast cf-ew71': 'COMFAST-CF-EW71',  # 带空格版本
    'cf-wa800v3': 'COMFAST CF-WA800V3',
    'tuya zigbee smart 24g millimeter': 'YT Tuya ZigBee Smart 24G Millimeter',
}

def main():
    print("=== 开始导入阿里订单数据 ===")

    # 读取 Excel
    print(f"\n读取 Excel: {EXCEL_PATH}")
    df = pd.read_excel(EXCEL_PATH)

    # 清理数据
    df = df.dropna(subset=['Order No.'])
    df['Client Name'] = df['Client Name'].fillna('').str.strip()
    df['Product'] = df['Product'].fillna('').str.strip()

    print(f"总行数: {len(df)}")
    print(f"唯一订单数: {df['Order No.'].nunique()}")

    # 连接数据库
    conn = sqlite3.connect(DB_PATH)
    cursor = conn.cursor()

    # 获取客户映射
    cursor.execute('SELECT id, name FROM customers')
    customers = {row[1].strip().lower(): row[0] for row in cursor.fetchall()}
    print(f"客户数量: {len(customers)}")

    # 获取产品映射
    cursor.execute('SELECT id, name FROM products')
    products = {row[1].strip().lower(): row[0] for row in cursor.fetchall()}
    print(f"产品数量: {len(products)}")

    # 获取已存在的订单号
    cursor.execute('SELECT order_code FROM orders WHERE platform = "ali_import"')
    existing_orders = {row[0] for row in cursor.fetchall()}
    print(f"已存在的阿里订单数: {len(existing_orders)}")

    # 按订单号分组
    grouped = df.groupby('Order No.')

    # 统计
    stats = {
        'total_orders': 0,
        'imported_orders': 0,
        'skipped_orders': 0,
        'total_items': 0,
        'imported_items': 0,
        'unmatched_customers': set(),
        'unmatched_products': set(),
    }

    for order_no, group in grouped:
        stats['total_orders'] += 1

        # 去重检查
        if order_no in existing_orders:
            stats['skipped_orders'] += 1
            continue

        # 获取订单信息
        first_row = group.iloc[0]
        client_name = first_row['Client Name'].strip()
        order_date = first_row['Date']

        # 计算订单总额（USD）
        order_amount_usd = group['Order Amount (USD)'].sum()
        order_amount_cny = order_amount_usd * EXCHANGE_RATE

        # 匹配客户
        customer_id = None
        client_name_lower = client_name.lower()
        if client_name_lower in customers:
            customer_id = customers[client_name_lower]
        else:
            stats['unmatched_customers'].add(client_name)

        # 解析日期
        if isinstance(order_date, str):
            try:
                created_at = datetime.strptime(order_date, '%Y-%m-%d').strftime('%Y-%m-%d %H:%M:%S')
            except:
                created_at = datetime.now().strftime('%Y-%m-%d %H:%M:%S')
        else:
            created_at = order_date.strftime('%Y-%m-%d %H:%M:%S') if pd.notna(order_date) else datetime.now().strftime('%Y-%m-%d %H:%M:%S')

        # 创建订单
        cursor.execute('''
            INSERT INTO orders (
                order_code, platform, customer_id, customer_name,
                order_status, payment_status, fulfillment_status,
                total_amount, subtotal, currency, exchange_rate,
                created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ''', (
            order_no,
            'ali_import',
            customer_id,
            client_name,
            3,  # 已付款
            2,  # 已支付
            3,  # 已完成
            order_amount_cny,
            order_amount_cny,
            'CNY',
            EXCHANGE_RATE,
            created_at,
            created_at,
        ))

        order_id = cursor.lastrowid
        stats['imported_orders'] += 1

        # 创建订单项
        for _, row in group.iterrows():
            stats['total_items'] += 1
            product_name = row['Product'].strip()
            quantity = int(row['Qty']) if pd.notna(row['Qty']) else 0
            unit_price_usd = float(row['Sales Unit Price (USD)']) if pd.notna(row['Sales Unit Price (USD)']) else 0
            cost_price_usd = float(row['Cost per Unit (USD)']) if pd.notna(row['Cost per Unit (USD)']) else 0

            unit_price_cny = unit_price_usd * EXCHANGE_RATE
            cost_price_cny = cost_price_usd * EXCHANGE_RATE
            total_amount_cny = unit_price_cny * quantity

            # 匹配产品
            product_id = None
            product_name_lower = product_name.lower()

            # 先检查别名映射
            if product_name_lower in PRODUCT_ALIASES:
                mapped_name = PRODUCT_ALIASES[product_name_lower].lower()
                if mapped_name in products:
                    product_id = products[mapped_name]
            else:
                # 直接匹配
                if product_name_lower in products:
                    product_id = products[product_name_lower]
                else:
                    stats['unmatched_products'].add(product_name)

            # 创建订单项
            cursor.execute('''
                INSERT INTO order_items (
                    order_id, product_id, product_name, quantity,
                    unit_price, subtotal, total_amount, cost_price, created_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            ''', (
                order_id,
                product_id,
                product_name,
                quantity,
                unit_price_cny,
                total_amount_cny,
                total_amount_cny,
                cost_price_cny,
                created_at,
            ))

            stats['imported_items'] += 1

    # 提交事务
    conn.commit()
    conn.close()

    # 输出统计
    print("\n=== 导入结果 ===")
    print(f"总订单数: {stats['total_orders']}")
    print(f"导入订单数: {stats['imported_orders']}")
    print(f"跳过订单数（已存在）: {stats['skipped_orders']}")
    print(f"总订单项数: {stats['total_items']}")
    print(f"导入订单项数: {stats['imported_items']}")

    if stats['unmatched_customers']:
        print(f"\n未匹配客户 ({len(stats['unmatched_customers'])}):")
        for c in sorted(stats['unmatched_customers']):
            print(f"  - {c}")

    if stats['unmatched_products']:
        print(f"\n未匹配产品 ({len(stats['unmatched_products'])}):")
        for p in sorted(stats['unmatched_products']):
            print(f"  - {p}")

    print("\n=== 导入完成 ===")

    return {
        'total_orders': stats['total_orders'],
        'imported_orders': stats['imported_orders'],
        'skipped_orders': stats['skipped_orders'],
        'imported_items': stats['imported_items'],
        'unmatched_customers': len(stats['unmatched_customers']),
        'unmatched_products': len(stats['unmatched_products']),
    }

if __name__ == '__main__':
    result = main()
    print(f"\n结果: {result}")
