#!/bin/bash
# ciciERP API 文档生成脚本
# 从代码注释中提取 API 文档并生成 Markdown 文件

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
ROUTES_DIR="$PROJECT_ROOT/crates/api/src/routes"
OUTPUT_FILE="$PROJECT_ROOT/docs/API.md"

echo "Generating API documentation..."
echo "Source: $ROUTES_DIR"
echo "Output: $OUTPUT_FILE"

# 创建输出目录
mkdir -p "$(dirname "$OUTPUT_FILE")"

# 初始化文档
cat > "$OUTPUT_FILE" << 'EOF'
# ciciERP API 文档

## 文档信息
- **版本**: v1.0
- **生成时间**: $(date '+%Y-%m-%d %H:%M:%S')
- **Base URL**: http://localhost:3000

---

## 通用说明

### 认证
部分 API 需要在请求头中携带 JWT Token：
```
Authorization: Bearer <token>
```

### 响应格式
所有 API 返回统一的 JSON 格式：

```json
{
  "code": 200,
  "message": "success",
  "data": { ... },
  "timestamp": 1709000000
}
```

### 错误响应
```json
{
  "code": 400,
  "message": "错误描述",
  "timestamp": 1709000000
}
```

### 分页格式
```json
{
  "code": 200,
  "message": "success",
  "data": {
    "items": [...],
    "pagination": {
      "page": 1,
      "page_size": 20,
      "total": 100,
      "total_pages": 5
    }
  }
}
```

---

EOF

# 提取 API 注释并生成文档
extract_api_docs() {
    local file=$1
    local module_name=$(basename "$file" .rs)

    # 跳过 mod.rs
    if [[ "$module_name" == "mod" ]]; then
        return
    fi

    # 模块标题映射
    local module_title=""
    case "$module_name" in
        products) module_title="产品管理" ;;
        suppliers) module_title="供应商管理" ;;
        customers) module_title="客户管理" ;;
        orders) module_title="订单管理" ;;
        inventory) module_title="库存管理" ;;
        health) module_title="系统健康检查" ;;
        *) module_title="$module_name" ;;
    esac

    echo "## $module_title" >> "$OUTPUT_FILE"
    echo "" >> "$OUTPUT_FILE"

    # 提取 @api 注释块
    local in_api_block=false
    local api_block=""

    while IFS= read -r line; do
        if [[ "$line" =~ ^///[[:space:]]*@api ]]; then
            in_api_block=true
            api_block="$line\n"
        elif [[ "$in_api_block" == true ]]; then
            if [[ "$line" =~ ^///[[:space:]]*@ ]]; then
                api_block="${api_block}${line}\n"
            elif [[ "$line" =~ ^pub[[:space:]]+async[[:space:]]+fn ]]; then
                # 结束 API 块，处理收集的内容
                process_api_block "$api_block"
                in_api_block=false
                api_block=""
            elif [[ -z "$line" ]] || [[ ! "$line" =~ ^/// ]]; then
                in_api_block=false
                api_block=""
            else
                api_block="${api_block}${line}\n"
            fi
        fi
    done < "$file"

    echo "" >> "$OUTPUT_FILE"
}

process_api_block() {
    local block=$1
    local method=""
    local path=""
    local desc=""
    local params=""
    local query=""
    local body=""
    local response=""
    local example=""

    # 解析各字段
    while IFS= read -r line; do
        if [[ "$line" =~ @api[[:space:]]+(GET|POST|PUT|DELETE|PATCH)[[:space:]]+(.+)$ ]]; then
            method="${BASH_REMATCH[1]}"
            path="${BASH_REMATCH[2]}"
        elif [[ "$line" =~ @desc[[:space:]]+(.+)$ ]]; then
            desc="${BASH_REMATCH[1]}"
        elif [[ "$line" =~ @param[[:space:]]+(.+)$ ]]; then
            params="${params}- **路径参数**: ${BASH_REMATCH[1]}\n"
        elif [[ "$line" =~ @query[[:space:]]+(.+)$ ]]; then
            query="${query}- ${BASH_REMATCH[1]}\n"
        elif [[ "$line" =~ @body[[:space:]]+(.+)$ ]]; then
            body="${BASH_REMATCH[1]}"
        elif [[ "$line" =~ @response[[:space:]]+(.+)$ ]]; then
            response="${response}- ${BASH_REMATCH[1]}\n"
        elif [[ "$line" =~ @example[[:space:]]+(.+)$ ]]; then
            example="${BASH_REMATCH[1]}"
        fi
    done <<< -e "$block"

    # 生成 Markdown
    if [[ -n "$method" && -n "$path" ]]; then
        echo "### $method $path" >> "$OUTPUT_FILE"
        echo "" >> "$OUTPUT_FILE"
        echo "$desc" >> "$OUTPUT_FILE"
        echo "" >> "$OUTPUT_FILE"

        if [[ -n "$params" ]]; then
            echo "**路径参数**:" >> "$OUTPUT_FILE"
            echo -e "$params" >> "$OUTPUT_FILE"
            echo "" >> "$OUTPUT_FILE"
        fi

        if [[ -n "$query" ]]; then
            echo "**查询参数**:" >> "$OUTPUT_FILE"
            echo -e "$query" >> "$OUTPUT_FILE"
            echo "" >> "$OUTPUT_FILE"
        fi

        if [[ -n "$body" ]]; then
            echo "**请求体**: \`$body\`" >> "$OUTPUT_FILE"
            echo "" >> "$OUTPUT_FILE"
        fi

        if [[ -n "$response" ]]; then
            echo "**响应**:" >> "$OUTPUT_FILE"
            echo -e "$response" >> "$OUTPUT_FILE"
            echo "" >> "$OUTPUT_FILE"
        fi

        if [[ -n "$example" ]]; then
            echo "**示例**:" >> "$OUTPUT_FILE"
            echo "\`\`\`bash" >> "$OUTPUT_FILE"
            echo "$example" >> "$OUTPUT_FILE"
            echo "\`\`\`" >> "$OUTPUT_FILE"
            echo "" >> "$OUTPUT_FILE"
        fi

        echo "---" >> "$OUTPUT_FILE"
        echo "" >> "$OUTPUT_FILE"
    fi
}

# 处理所有路由文件
for file in "$ROUTES_DIR"/*.rs; do
    if [[ -f "$file" ]]; then
        echo "Processing: $file"
        extract_api_docs "$file"
    fi
done

# 更新生成时间
sed -i "s/\$(date '+%Y-%m-%d %H:%M:%S')/$(date '+%Y-%m-%d %H:%M:%S')/" "$OUTPUT_FILE"

echo "API documentation generated: $OUTPUT_FILE"
echo "Done!"
