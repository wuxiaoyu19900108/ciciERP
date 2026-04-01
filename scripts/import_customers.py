#!/usr/bin/env python3
"""
客户信息导入脚本
从 Excel 文件导入客户信息到 ciciERP 数据库
"""

import pandas as pd
import sqlite3
import re
from datetime import datetime
from typing import Optional, Tuple

# 配置
EXCEL_FILE = '/home/wxy/.xiaozhi/files/file_1774538047517_cf854eaa_customers.xlsx.xlsx'
DB_FILE = 'data/cicierp.db'

def is_email(contact: str) -> bool:
    """判断是否为邮箱"""
    if not contact or pd.isna(contact):
        return False
    email_pattern = r'^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$'
    return bool(re.match(email_pattern, str(contact).strip()))

def is_phone(contact: str) -> bool:
    """判断是否为电话号码"""
    if not contact or pd.isna(contact):
        return False
    # 移除空格和常见分隔符后检查是否主要是数字
    cleaned = re.sub(r'[\s\-\(\)\+]', '', str(contact))
    return cleaned.isdigit() and len(cleaned) >= 6

def generate_customer_code(conn: sqlite3.Connection) -> str:
    """生成客户编码 CUS-YYYYMMDD-XXXX"""
    today = datetime.now().strftime('%Y%m%d')
    cursor = conn.cursor()

    # 查找今天已有的最大序号
    cursor.execute("""
        SELECT customer_code FROM customers
        WHERE customer_code LIKE ?
        ORDER BY customer_code DESC LIMIT 1
    """, (f'CUS-{today}-%',))

    result = cursor.fetchone()
    if result:
        last_num = int(result[0].split('-')[-1])
        new_num = last_num + 1
    else:
        new_num = 1

    return f'CUS-{today}-{new_num:04d}'

def customer_exists(conn: sqlite3.Connection, name: str) -> Optional[int]:
    """检查客户是否已存在，返回客户ID或None"""
    cursor = conn.cursor()
    cursor.execute("SELECT id FROM customers WHERE name = ? AND deleted_at IS NULL", (name,))
    result = cursor.fetchone()
    return result[0] if result else None

def update_customer(conn: sqlite3.Connection, customer_id: int, mobile: Optional[str], email: Optional[str], source: Optional[str]):
    """更新客户缺失信息"""
    cursor = conn.cursor()

    # 获取当前客户信息
    cursor.execute("SELECT mobile, email, source FROM customers WHERE id = ?", (customer_id,))
    current = cursor.fetchone()

    updates = []
    params = []

    # 只更新缺失的字段
    if not current[0] and mobile:
        updates.append("mobile = ?")
        params.append(mobile)
    if not current[1] and email:
        updates.append("email = ?")
        params.append(email)
    if not current[2] and source:
        updates.append("source = ?")
        params.append(source)

    if updates:
        updates.append("updated_at = datetime('now')")
        params.append(customer_id)
        sql = f"UPDATE customers SET {', '.join(updates)} WHERE id = ?"
        cursor.execute(sql, params)
        conn.commit()
        return True
    return False

def create_customer(conn: sqlite3.Connection, name: str, mobile: Optional[str],
                   email: Optional[str], source: str) -> int:
    """创建新客户，返回客户ID"""
    cursor = conn.cursor()
    customer_code = generate_customer_code(conn)

    cursor.execute("""
        INSERT INTO customers (customer_code, name, mobile, email, source, status, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, 1, datetime('now'), datetime('now'))
    """, (customer_code, name, mobile, email, source))

    conn.commit()
    return cursor.lastrowid

def create_address(conn: sqlite3.Connection, customer_id: int, receiver_name: str,
                   country: str, address: str, phone: Optional[str] = None):
    """创建收货地址"""
    cursor = conn.cursor()

    # 检查是否已有地址
    cursor.execute("SELECT id FROM customer_addresses WHERE customer_id = ?", (customer_id,))
    if cursor.fetchone():
        return None

    cursor.execute("""
        INSERT INTO customer_addresses
        (customer_id, receiver_name, receiver_phone, country, address, is_default, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, 1, datetime('now'), datetime('now'))
    """, (customer_id, receiver_name, phone or '', country, address))

    conn.commit()
    return cursor.lastrowid

def main():
    # 读取 Excel
    print(f"读取 Excel 文件: {EXCEL_FILE}")
    df = pd.read_excel(EXCEL_FILE)
    print(f"共 {len(df)} 条记录")

    # 连接数据库
    conn = sqlite3.connect(DB_FILE)

    # 统计
    stats = {
        'total': len(df),
        'created': 0,
        'updated': 0,
        'skipped': 0,
        'addresses_created': 0,
        'errors': 0
    }

    # 处理每一行
    for idx, row in df.iterrows():
        try:
            name = str(row['Client Name']).strip() if pd.notna(row['Client Name']) else None
            if not name:
                stats['skipped'] += 1
                continue

            contact = str(row['Contact Info']).strip() if pd.notna(row['Contact Info']) else None
            source = str(row['Source']).strip() if pd.notna(row['Source']) else 'Unknown'
            country = str(row['Country']).strip() if pd.notna(row['Country']) else ''
            shipping_address = str(row['Shipping Address']).strip() if pd.notna(row['Shipping Address']) else None

            # 判断联系方式类型
            mobile = None
            email = None
            if contact:
                if is_email(contact):
                    email = contact
                elif is_phone(contact):
                    mobile = contact
                else:
                    # 无法判断，尝试作为邮箱处理
                    if '@' in contact:
                        email = contact
                    else:
                        mobile = contact

            # 检查客户是否存在
            existing_id = customer_exists(conn, name)

            if existing_id:
                # 更新缺失信息
                if update_customer(conn, existing_id, mobile, email, source):
                    stats['updated'] += 1
                else:
                    stats['skipped'] += 1

                customer_id = existing_id
            else:
                # 创建新客户
                customer_id = create_customer(conn, name, mobile, email, source)
                stats['created'] += 1

            # 创建收货地址（如果有）
            if shipping_address and shipping_address != 'nan' and country:
                addr_id = create_address(conn, customer_id, name, country, shipping_address, mobile)
                if addr_id:
                    stats['addresses_created'] += 1

        except Exception as e:
            print(f"处理第 {idx + 1} 行时出错: {e}")
            stats['errors'] += 1

    conn.close()

    # 输出统计
    print("\n=== 导入完成 ===")
    print(f"总记录数: {stats['total']}")
    print(f"新建客户: {stats['created']}")
    print(f"更新客户: {stats['updated']}")
    print(f"跳过记录: {stats['skipped']}")
    print(f"新建地址: {stats['addresses_created']}")
    print(f"错误数: {stats['errors']}")

    return stats

if __name__ == '__main__':
    main()
