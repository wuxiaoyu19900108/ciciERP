//! 认证相关模板数据结构

/// 登录表单数据
#[derive(Debug, Clone, serde::Deserialize)]
pub struct LoginForm {
    pub username: String,
    pub password: String,
}
