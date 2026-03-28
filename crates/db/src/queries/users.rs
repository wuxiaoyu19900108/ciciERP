//! 用户数据库查询

use sqlx::SqlitePool;

use cicierp_models::user::{CreateUserRequest, Role, RoleInfo, UpdateUserRequest, User, UserInfo};

/// 用户查询器
pub struct UserQueries<'a> {
    pool: &'a SqlitePool,
}

impl<'a> UserQueries<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    /// 根据ID获取用户
    pub async fn get_by_id(&self, id: i64) -> sqlx::Result<Option<User>> {
        sqlx::query_as::<_, User>(
            "SELECT id, username, password_hash, email, mobile, real_name, avatar,
                    status, last_login_at, last_login_ip, created_at, updated_at, deleted_at
             FROM users WHERE id = ? AND deleted_at IS NULL"
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await
    }

    /// 根据用户名获取用户
    pub async fn get_by_username(&self, username: &str) -> sqlx::Result<Option<User>> {
        sqlx::query_as::<_, User>(
            "SELECT id, username, password_hash, email, mobile, real_name, avatar,
                    status, last_login_at, last_login_ip, created_at, updated_at, deleted_at
             FROM users WHERE username = ? AND deleted_at IS NULL"
        )
        .bind(username)
        .fetch_optional(self.pool)
        .await
    }

    /// 根据邮箱获取用户
    pub async fn get_by_email(&self, email: &str) -> sqlx::Result<Option<User>> {
        sqlx::query_as::<_, User>(
            "SELECT id, username, password_hash, email, mobile, real_name, avatar,
                    status, last_login_at, last_login_ip, created_at, updated_at, deleted_at
             FROM users WHERE email = ? AND deleted_at IS NULL"
        )
        .bind(email)
        .fetch_optional(self.pool)
        .await
    }

    /// 根据手机号获取用户
    pub async fn get_by_mobile(&self, mobile: &str) -> sqlx::Result<Option<User>> {
        sqlx::query_as::<_, User>(
            "SELECT id, username, password_hash, email, mobile, real_name, avatar,
                    status, last_login_at, last_login_ip, created_at, updated_at, deleted_at
             FROM users WHERE mobile = ? AND deleted_at IS NULL"
        )
        .bind(mobile)
        .fetch_optional(self.pool)
        .await
    }

    /// 创建用户
    pub async fn create(&self, req: &CreateUserRequest, password_hash: &str) -> sqlx::Result<User> {
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let status = req.status.unwrap_or(1);

        let result = sqlx::query(
            "INSERT INTO users (username, password_hash, email, mobile, real_name, avatar, status, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&req.username)
        .bind(password_hash)
        .bind(&req.email)
        .bind(&req.mobile)
        .bind(&req.real_name)
        .bind(&req.avatar)
        .bind(status)
        .bind(&now)
        .bind(&now)
        .execute(self.pool)
        .await?;

        let id = result.last_insert_rowid();

        // 分配角色
        if let Some(role_ids) = &req.role_ids {
            for role_id in role_ids {
                sqlx::query(
                    "INSERT INTO user_roles (user_id, role_id, created_at) VALUES (?, ?, ?)"
                )
                .bind(id)
                .bind(role_id)
                .bind(&now)
                .execute(self.pool)
                .await?;
            }
        }

        self.get_by_id(id).await.map(|u| u.unwrap())
    }

    /// 更新用户
    pub async fn update(&self, id: i64, req: &UpdateUserRequest) -> sqlx::Result<Option<User>> {
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

        // 先检查用户是否存在
        let user = self.get_by_id(id).await?;
        if user.is_none() {
            return Ok(None);
        }

        sqlx::query(
            "UPDATE users SET
                email = COALESCE(?, email),
                mobile = COALESCE(?, mobile),
                real_name = COALESCE(?, real_name),
                avatar = COALESCE(?, avatar),
                status = COALESCE(?, status),
                updated_at = ?
             WHERE id = ?"
        )
        .bind(&req.email)
        .bind(&req.mobile)
        .bind(&req.real_name)
        .bind(&req.avatar)
        .bind(req.status)
        .bind(&now)
        .bind(id)
        .execute(self.pool)
        .await?;

        // 更新角色
        if let Some(role_ids) = &req.role_ids {
            // 删除旧角色
            sqlx::query("DELETE FROM user_roles WHERE user_id = ?")
                .bind(id)
                .execute(self.pool)
                .await?;

            // 添加新角色
            for role_id in role_ids {
                sqlx::query(
                    "INSERT INTO user_roles (user_id, role_id, created_at) VALUES (?, ?, ?)"
                )
                .bind(id)
                .bind(role_id)
                .bind(&now)
                .execute(self.pool)
                .await?;
            }
        }

        self.get_by_id(id).await
    }

    /// 更新密码
    pub async fn update_password(&self, id: i64, password_hash: &str) -> sqlx::Result<bool> {
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

        let result = sqlx::query(
            "UPDATE users SET password_hash = ?, updated_at = ? WHERE id = ? AND deleted_at IS NULL"
        )
        .bind(password_hash)
        .bind(&now)
        .bind(id)
        .execute(self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// 更新最后登录时间
    pub async fn update_last_login(&self, id: i64, ip: Option<&str>) -> sqlx::Result<()> {
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

        sqlx::query(
            "UPDATE users SET last_login_at = ?, last_login_ip = ?, updated_at = ? WHERE id = ?"
        )
        .bind(&now)
        .bind(ip)
        .bind(&now)
        .bind(id)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    /// 删除用户（软删除）
    pub async fn delete(&self, id: i64) -> sqlx::Result<bool> {
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

        let result = sqlx::query(
            "UPDATE users SET deleted_at = ?, updated_at = ? WHERE id = ? AND deleted_at IS NULL"
        )
        .bind(&now)
        .bind(&now)
        .bind(id)
        .execute(self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// 获取用户列表
    pub async fn list(
        &self,
        page: u32,
        page_size: u32,
        keyword: Option<&str>,
        status: Option<i64>,
    ) -> sqlx::Result<(Vec<User>, u64)> {
        let offset = (page - 1) * page_size;

        // 构建查询条件
        let mut conditions = vec!["deleted_at IS NULL"];
        let mut params: Vec<String> = vec![];

        if let Some(kw) = keyword {
            conditions.push("(username LIKE ? OR real_name LIKE ? OR email LIKE ?)");
            let pattern = format!("%{}%", kw);
            params.push(pattern.clone());
            params.push(pattern.clone());
            params.push(pattern);
        }

        if let Some(s) = status {
            conditions.push("status = ?");
            params.push(s.to_string());
        }

        let where_clause = conditions.join(" AND ");

        // 查询总数
        let count_sql = format!("SELECT COUNT(*) FROM users WHERE {}", where_clause);
        let (total,): (i64,) = if params.is_empty() {
            sqlx::query_as(&count_sql)
                .fetch_one(self.pool)
                .await?
        } else {
            let mut query = sqlx::query_as::<_, (i64,)>(&count_sql);
            for param in &params {
                query = query.bind(param);
            }
            query.fetch_one(self.pool).await?
        };

        // 查询列表
        let list_sql = format!(
            "SELECT id, username, password_hash, email, mobile, real_name, avatar,
                    status, last_login_at, last_login_ip, created_at, updated_at, deleted_at
             FROM users WHERE {} ORDER BY id DESC LIMIT ? OFFSET ?",
            where_clause
        );

        let users: Vec<User> = if params.is_empty() {
            sqlx::query_as(&list_sql)
                .bind(page_size as i32)
                .bind(offset as i32)
                .fetch_all(self.pool)
                .await?
        } else {
            let mut query = sqlx::query_as::<_, User>(&list_sql);
            for param in &params {
                query = query.bind(param);
            }
            query = query.bind(page_size as i32).bind(offset as i32);
            query.fetch_all(self.pool).await?
        };

        Ok((users, total as u64))
    }

    /// 获取用户角色
    pub async fn get_user_roles(&self, user_id: i64) -> sqlx::Result<Vec<Role>> {
        sqlx::query_as::<_, Role>(
            "SELECT r.id, r.name, r.code, r.description, r.permissions, r.status, r.created_at, r.updated_at
             FROM roles r
             INNER JOIN user_roles ur ON r.id = ur.role_id
             WHERE ur.user_id = ?"
        )
        .bind(user_id)
        .fetch_all(self.pool)
        .await
    }

    /// 获取用户角色简要信息
    pub async fn get_user_role_info(&self, user_id: i64) -> sqlx::Result<Vec<RoleInfo>> {
        sqlx::query_as::<_, RoleInfo>(
            "SELECT r.id, r.name, r.code
             FROM roles r
             INNER JOIN user_roles ur ON r.id = ur.role_id
             WHERE ur.user_id = ?"
        )
        .bind(user_id)
        .fetch_all(self.pool)
        .await
    }

    /// 获取用户权限列表
    pub async fn get_user_permissions(&self, user_id: i64) -> sqlx::Result<Vec<String>> {
        // 获取用户所有角色的权限
        let roles = self.get_user_roles(user_id).await?;
        let mut permissions = std::collections::HashSet::new();

        for role in roles {
            if let Ok(perms) = serde_json::from_str::<Vec<String>>(&role.permissions) {
                for perm in perms {
                    permissions.insert(perm);
                }
            }
        }

        Ok(permissions.into_iter().collect())
    }

    /// 检查用户是否有特定权限
    pub async fn has_permission(&self, user_id: i64, permission: &str) -> sqlx::Result<bool> {
        let permissions = self.get_user_permissions(user_id).await?;

        // 检查是否有通配符权限
        if permissions.contains(&"*".to_string()) {
            return Ok(true);
        }

        // 检查精确匹配
        if permissions.contains(&permission.to_string()) {
            return Ok(true);
        }

        // 检查模块通配符 (如 users:*)
        if let Some(colon_pos) = permission.find(':') {
            let module_perm = format!("{}:*", &permission[..colon_pos]);
            if permissions.contains(&module_perm) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// 检查用户是否有特定角色
    pub async fn has_role(&self, user_id: i64, role_code: &str) -> sqlx::Result<bool> {
        let roles = self.get_user_roles(user_id).await?;
        Ok(roles.iter().any(|r| r.code == role_code))
    }
}

/// 角色查询器
pub struct RoleQueries<'a> {
    pool: &'a SqlitePool,
}

impl<'a> RoleQueries<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    /// 获取所有角色
    pub async fn list(&self) -> sqlx::Result<Vec<Role>> {
        sqlx::query_as::<_, Role>(
            "SELECT id, name, code, description, permissions, status, created_at, updated_at
             FROM roles WHERE status = 1 ORDER BY id"
        )
        .fetch_all(self.pool)
        .await
    }

    /// 根据ID获取角色
    pub async fn get_by_id(&self, id: i64) -> sqlx::Result<Option<Role>> {
        sqlx::query_as::<_, Role>(
            "SELECT id, name, code, description, permissions, status, created_at, updated_at
             FROM roles WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await
    }

    /// 根据code获取角色
    pub async fn get_by_code(&self, code: &str) -> sqlx::Result<Option<Role>> {
        sqlx::query_as::<_, Role>(
            "SELECT id, name, code, description, permissions, status, created_at, updated_at
             FROM roles WHERE code = ?"
        )
        .bind(code)
        .fetch_optional(self.pool)
        .await
    }
}
