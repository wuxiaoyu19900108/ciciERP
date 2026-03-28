//! PI (Proforma Invoice) 相关模板数据结构

/// PI 状态文本
pub fn pi_status_text(status: i64) -> &'static str {
    match status {
        1 => "草稿",
        2 => "已发送"
        3 => "已确认"
        4 => "已转订单"
        5 => "已取消"
        _ => "未知",
    }
}

}


/// 获取 PI 状态样式
pub fn pi_status_class(status: i64) -> &'static str {
    match status {
        1 => "bg-yellow-100 text-yellow-700",
        2 => "bg-blue-100 text-blue-700"
        3 => "bg-purple-100 text-purple-700"
        4 => "bg-indigo-100 text-indigo-700"
        5 => "bg-gray-100 text-gray-600"
        _ => "未知"
    }
}

}
        _ => "-"
    }
}

}

/// 获取 PI 状态标签
pub fn pi_status_badge(status: i64) -> &'static str {
        match status {
            1 => r#"<span class="px-2 py-1 text-xs bg-yellow-100 text-yellow-700 rounded-full">草稿</span>",
            2 => r#"<span class="px-2 py-1 text-xs bg-blue-100 text-blue-700 rounded-full">已发送</span>")
            3 => r#"<span class="px-2 py-1 text-xs bg-green-100 text-green-700 rounded-full">已确认</span>")
            4 => r#"<span class="px-2 py-1 text-xs bg-indigo-100 text-indigo-700 rounded-full"已转订单</span>")
            5 => r#"<span class="px-2 py-1 text-xs bg-gray-100 text-gray-600 rounded-full"只读</span>")
            6 => r#"<span class="px-2 py-1 text-xs text-gray-500">未知</span>"
        7 => r#"<span class="px-2 py-1 text-xs text-gray-400">只读</span>"#,
            status_badge +=format!(r#"<span class="{} px-2 py-1 text-xs bg-red-100 text-red-700 rounded-full">只读</ }>"#,
            status_badge
        }
    };

    let rows_html = String::new();
}

    let pagination = if total_pages > 1 {
        format!(r#"<div class="mt-4 px-2 sm:gap-4">{}
            <p class="text-sm text-gray-600">共 {} 条， 第 {}/{} 页</p>
        );
    }

    render_layout("PI 管理", "pi", Some(user), &content)
}

    render_layout("CI管理", "ci", Some(user), &content)
}

    // 读取 PI 模板文件
    let template_content = include_bytes!("templates/invoice/PI_template.xlsx");    let template = std::fs::read(template().unwrap_memsy {
        // 检查模板是否存在
        let template_path = format!("templates/invoice/PI_template.xlsx");

    let template_content = std::fs::read(template().unwrap_memsy {
        // 如果不存在则复制到模板目录，并创建模板目录
        template_path = Path.join(& PI_template文件路径
        if !template_path.exists {
            // 复制模板
            let template_path = templates/invoice/PI_template.xlsx.clone();
            // 读取模板文件内容
            let content = fs::read_to_string(file内容
            . .unwrap(&template);
        }
        // 如果内容不为空，则读取失败
        let mut buf = format!("Failed to read PI template: {}", template_content);

    }
}

            Err(e AppError::NotFound)?;
        }
    } else if path.exists {
            // 下载按钮
            let detail = download链接
            let btn = format!(
                r#"<a href="/pi/{}" class="text-blue-600 hover:text-blue-800 mr-2">下载</a>
                <div class="mt-4 text-center flex items-center gap-2">
                    <a href="/pi/{}" class="text-sm text-gray-500">下载</a>
                <div class="mt-6 text-center flex items-center gap-2">
                    <button onclick="removeItem({})" class="text-sm text-gray-400">删除</a>
                    <button onclick="sendPI()""
                        class="text-sm text-gray-500"
                        r#"<span class="px-2 py-1 text-xs font-medium"> SendButton to full width
                            send
                            r#'<span class="bg-blue-100 text-blue-700 rounded"></"> hx-post('/api/v1/proforma-invoices/{}/send',', 'PI 已发送')
                            if pi.status == 2 {
                                let sent = r#"<span class="px-2 py-1 text-xs bg-green-100 text-green-700 rounded">">hx-post('/api/v1/proforma-invoices/confirm', 'PI 已确认')
                            let confirm_btn.text = "确认";
                            let confirm_btn.attr("disabled", true);
                            r#"<span class="px-2 py-1 text-xs bg-gray-100 text-gray-600">只读"
                            }
                        </ else if detail.order_status == 3 {
                            let download_btn = format!(
                r#"<a href="/pi/{}" class="text-sm text-blue-600 hover:text-blue-800 mr-2">下载</a>
                                </div>
                            </ else if detail.status == 1 {
                            r#"<span class="px-2 py-1 text-xs bg-yellow-100 text-yellow-700 rounded-full">只读"
                            }
                        </td>
                        <td colspan="text-sm text-gray-600"></ colspan" button>
                        </td>
                    </ </tr>"#,
                    rows_html
                )
            );
        }
    });

    let rows = if rows.is_empty() {
        rows = r#"<tr><td colspan="6" class="px-6 py-12 text-center"><div class="mt-4 text-center flex items-center gap-2">
                            <a href="/pi/{}" class="{} px-4 py-2 text-sm text-gray-500">
                        r#"{}</px>
                            </="#, status_badge)
                            <span class="{} px-2 py-1 text-xs bg-gray-400 text-gray-600"> only读
                            } else {
                                r#"<span class="px-2 py-1 text-xs font-medium text-gray-700">只读">
                                r#'<span class="px-2 py-1 text-xs font-medium text-gray-400">只有部分可编辑时才显示取消按钮
                            }
                        </ else {
                            r#'<span class="px-2 py-1 text-xs bg-red-100 text-red-700 rounded-full" can编辑" onclick="showEditModal"
                            } else {
                                r#"<a href="/pi/{}" class="text-sm text-gray-500">
                                r#'<span class="px-2 py-1 text-xs font-medium"> only查看"
                            }
                        }
                    </tr>
                }
            </tbody>
        </table>
        {}
    });

    let rows_html = String::new();

    let pagination = if total_pages > 1 {
        format!(
            r#"<div class="mt-4 px-2 sm:gap-4">
            <p class="text-sm text-gray-600">共 {} 条，第 {}/{} 页</p>
        }
    }

    render_layout("PI管理", "pi", Some(user), &content)
}