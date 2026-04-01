# -*- coding: utf-8 -*-

import streamlit as st
import pandas as pd
from pathlib import Path

# 页面设置
st.set_page_config(page_title="Mini ERP - Product Prices", layout="wide")

# 数据文件名（放在和 app.py 同一个文件夹里）
DATA_FILE = "products.xls"   # 你的 Excel 文件
HEADER_SKIP_ROWS = 2         # 如果第 1 行是汇率、第 2 行是空行，就填 2；如果第一行就是表头就改成 0


@st.cache_data
def load_data() -> pd.DataFrame:
    """
    从当前脚本所在目录读取 Excel 文件
    """
    # 让路径始终是 “app.py 所在的文件夹 + DATA_FILE”
    path = Path(__file__).with_name(DATA_FILE)

    if not path.exists():
        st.error(f"找不到数据文件：{path.resolve()}")
        return pd.DataFrame()

    try:
        # 读取 .xls 文件。如果以后改成 .xlsx，也可以继续用 read_excel
        df = pd.read_excel(path, skiprows=HEADER_SKIP_ROWS)
    except Exception as e:
        st.error(f"读取 {path.name} 时出错：{e}")
        return pd.DataFrame()

    # 去掉全空的列
    df = df.dropna(axis=1, how="all")
    return df


def main():
    st.title("🧾 Mini ERP - 商品价格表")

    df = load_data()
    if df.empty:
        # 如果没有数据就直接停止后面的渲染
        st.stop()

    # 根据你的表头列名设置（可以根据需要调整）
    name_col = "Product Name"
    model_col = "Model"
    price_col = "Selling Price (USD)"
    cost_col = "Cost (USD)"
    supplier_col = "Supplier"
    stock_col = "Stock Qty"

    # ---------------- 侧边栏筛选 ----------------
    st.sidebar.header("筛选条件")

    # 关键字搜索
    keyword = st.sidebar.text_input("按名称 / 型号搜索（模糊）：", "")

    # 供应商多选
    if supplier_col in df.columns:
        suppliers = sorted(df[supplier_col].dropna().unique().tolist())
        supplier_filter = st.sidebar.multiselect("供应商：", options=suppliers, default=[])
    else:
        supplier_filter = []

    # 价格区间滑块
    if price_col in df.columns and pd.api.types.is_numeric_dtype(df[price_col]):
        min_price = float(df[price_col].min())
        max_price = float(df[price_col].max())
        price_range = st.sidebar.slider(
            "按销售价格范围过滤 (USD)：",
            min_value=min_price,
            max_value=max_price,
            value=(min_price, max_price),
            step=max(0.1, round((max_price - min_price) / 100, 2)),
        )
    else:
        price_range = None

    # ---------------- 应用筛选 ----------------
    filtered = df.copy()

    # 模糊搜索
    if keyword:
        kw = keyword.lower()
        cols_for_search = []
        for col in [name_col, model_col]:
            if col in filtered.columns:
                cols_for_search.append(col)

        if cols_for_search:
            mask = False
            for col in cols_for_search:
                mask = mask | filtered[col].astype(str).str.lower().str.contains(kw)
            filtered = filtered[mask]

    # 按供应商过滤
    if supplier_filter and supplier_col in filtered.columns:
        filtered = filtered[filtered[supplier_col].isin(supplier_filter)]

    # 按价格区间过滤
    if (
        price_range
        and price_col in filtered.columns
        and pd.api.types.is_numeric_dtype(filtered[price_col])
    ):
        filtered = filtered[
            (filtered[price_col] >= price_range[0])
            & (filtered[price_col] <= price_range[1])
        ]

    # ---------------- 顶部统计信息 ----------------
    cols = st.columns(4)
    cols[0].metric("商品数量", len(filtered))

    if stock_col in filtered.columns and pd.api.types.is_numeric_dtype(filtered[stock_col]):
        total_stock = int(filtered[stock_col].fillna(0).sum())
        cols[1].metric("库存总数", total_stock)

    if price_col in filtered.columns and pd.api.types.is_numeric_dtype(filtered[price_col]):
        avg_price = filtered[price_col].mean()
        cols[2].metric("平均销售价 (USD)", f"{avg_price:.2f}")

    if (
        price_col in filtered.columns
        and stock_col in filtered.columns
        and pd.api.types.is_numeric_dtype(filtered[price_col])
        and pd.api.types.is_numeric_dtype(filtered[stock_col])
    ):
        stock_value = (filtered[price_col] * filtered[stock_col]).sum()
        cols[3].metric("理论库存价值 (USD)", f"{stock_value:,.2f}")

    st.markdown("---")

    # ---------------- 商品明细表 ----------------
    st.subheader("商品明细")

    st.dataframe(
        filtered,
        use_container_width=True,
        height=600,
    )


if __name__ == "__main__":
    main()
