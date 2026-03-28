//! 物流数据库查询

use sqlx::SqlitePool;
use tracing::info;

use cicierp_models::logistics::{
    AddTrackingRequest, CreateLogisticsCompanyRequest, CreateShipmentRequest,
    LogisticsCompany, Shipment, ShipmentDetail, ShipmentListItem, ShipmentQuery,
    ShipmentTracking, UpdateLogisticsCompanyRequest, UpdateShipmentRequest,
};

/// 物流公司查询器
pub struct LogisticsCompanyQueries<'a> {
    pool: &'a SqlitePool,
}

impl<'a> LogisticsCompanyQueries<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    /// 获取所有物流公司
    pub async fn list(&self) -> sqlx::Result<Vec<LogisticsCompany>> {
        sqlx::query_as::<_, LogisticsCompany>(
            "SELECT id, code, name, name_en, service_type, api_code, api_config,
                    contact_phone, contact_email, website, tracking_url_template, status,
                    created_at, updated_at
             FROM logistics_companies WHERE status = 1 ORDER BY id"
        )
        .fetch_all(self.pool)
        .await
    }

    /// 根据ID获取
    pub async fn get_by_id(&self, id: i64) -> sqlx::Result<Option<LogisticsCompany>> {
        sqlx::query_as::<_, LogisticsCompany>(
            "SELECT id, code, name, name_en, service_type, api_code, api_config,
                    contact_phone, contact_email, website, tracking_url_template, status,
                    created_at, updated_at
             FROM logistics_companies WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await
    }

    /// 根据code获取
    pub async fn get_by_code(&self, code: &str) -> sqlx::Result<Option<LogisticsCompany>> {
        sqlx::query_as::<_, LogisticsCompany>(
            "SELECT id, code, name, name_en, service_type, api_code, api_config,
                    contact_phone, contact_email, website, tracking_url_template, status,
                    created_at, updated_at
             FROM logistics_companies WHERE code = ?"
        )
        .bind(code)
        .fetch_optional(self.pool)
        .await
    }

    /// 创建物流公司
    pub async fn create(&self, req: &CreateLogisticsCompanyRequest) -> sqlx::Result<LogisticsCompany> {
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

        let result = sqlx::query(
            "INSERT INTO logistics_companies (code, name, name_en, service_type, api_code,
             api_config, contact_phone, contact_email, website, tracking_url_template, status,
             created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1, ?, ?)"
        )
        .bind(&req.code)
        .bind(&req.name)
        .bind(&req.name_en)
        .bind(&req.service_type)
        .bind(&req.api_code)
        .bind(&req.api_config)
        .bind(&req.contact_phone)
        .bind(&req.contact_email)
        .bind(&req.website)
        .bind(&req.tracking_url_template)
        .bind(&now)
        .bind(&now)
        .execute(self.pool)
        .await?;

        info!("Logistics company created: {}", req.code);
        self.get_by_id(result.last_insert_rowid()).await.map(|o| o.unwrap())
    }

    /// 更新物流公司
    pub async fn update(&self, id: i64, req: &UpdateLogisticsCompanyRequest) -> sqlx::Result<Option<LogisticsCompany>> {
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

        sqlx::query(
            "UPDATE logistics_companies SET
                name = COALESCE(?, name),
                name_en = COALESCE(?, name_en),
                service_type = COALESCE(?, service_type),
                api_code = COALESCE(?, api_code),
                api_config = COALESCE(?, api_config),
                contact_phone = COALESCE(?, contact_phone),
                contact_email = COALESCE(?, contact_email),
                website = COALESCE(?, website),
                tracking_url_template = COALESCE(?, tracking_url_template),
                status = COALESCE(?, status),
                updated_at = ?
             WHERE id = ?"
        )
        .bind(&req.name)
        .bind(&req.name_en)
        .bind(&req.service_type)
        .bind(&req.api_code)
        .bind(&req.api_config)
        .bind(&req.contact_phone)
        .bind(&req.contact_email)
        .bind(&req.website)
        .bind(&req.tracking_url_template)
        .bind(req.status)
        .bind(&now)
        .bind(id)
        .execute(self.pool)
        .await?;

        self.get_by_id(id).await
    }

    /// 删除物流公司（设置状态为禁用）
    pub async fn delete(&self, id: i64) -> sqlx::Result<bool> {
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

        let result = sqlx::query(
            "UPDATE logistics_companies SET status = 0, updated_at = ? WHERE id = ?"
        )
        .bind(&now)
        .bind(id)
        .execute(self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }
}

/// 发货单查询器
pub struct ShipmentQueries<'a> {
    pool: &'a SqlitePool,
}

impl<'a> ShipmentQueries<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    /// 生成发货单号
    fn generate_shipment_code() -> String {
        let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S");
        let random: u32 = rand::random::<u32>() % 10000;
        format!("SH{}{:04}", timestamp, random)
    }

    /// 根据ID获取发货单
    pub async fn get_by_id(&self, id: i64) -> sqlx::Result<Option<Shipment>> {
        sqlx::query_as::<_, Shipment>(
            "SELECT id, shipment_code, order_id, logistics_id, logistics_name, tracking_number,
                    receiver_name, receiver_phone, receiver_address, package_weight, package_volume,
                    package_items, package_count, shipping_fee, actual_shipping_fee, estimated_arrival,
                    actual_arrival, status, shipping_note, ship_time, created_at, updated_at
             FROM shipments WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await
    }

    /// 根据发货单号获取
    pub async fn get_by_code(&self, shipment_code: &str) -> sqlx::Result<Option<Shipment>> {
        sqlx::query_as::<_, Shipment>(
            "SELECT id, shipment_code, order_id, logistics_id, logistics_name, tracking_number,
                    receiver_name, receiver_phone, receiver_address, package_weight, package_volume,
                    package_items, package_count, shipping_fee, actual_shipping_fee, estimated_arrival,
                    actual_arrival, status, shipping_note, ship_time, created_at, updated_at
             FROM shipments WHERE shipment_code = ?"
        )
        .bind(shipment_code)
        .fetch_optional(self.pool)
        .await
    }

    /// 获取发货单详情（含轨迹）
    pub async fn get_detail(&self, id: i64) -> sqlx::Result<Option<ShipmentDetail>> {
        let shipment = self.get_by_id(id).await?;
        match shipment {
            Some(shipment) => {
                let tracking = self.get_tracking(id).await?;
                Ok(Some(ShipmentDetail { shipment, tracking }))
            }
            None => Ok(None),
        }
    }

    /// 获取物流轨迹
    pub async fn get_tracking(&self, shipment_id: i64) -> sqlx::Result<Vec<ShipmentTracking>> {
        sqlx::query_as::<_, ShipmentTracking>(
            "SELECT id, shipment_id, tracking_time, tracking_status, tracking_description,
                    location, raw_data, created_at
             FROM shipment_tracking WHERE shipment_id = ? ORDER BY tracking_time DESC"
        )
        .bind(shipment_id)
        .fetch_all(self.pool)
        .await
    }

    /// 创建发货单
    pub async fn create(&self, req: &CreateShipmentRequest, receiver_info: &(String, String, String)) -> sqlx::Result<Shipment> {
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let shipment_code = Self::generate_shipment_code();
        let package_items_json = serde_json::to_string(&req.package_items).unwrap_or_else(|_| "[]".to_string());

        let result = sqlx::query(
            "INSERT INTO shipments (shipment_code, order_id, logistics_id, logistics_name,
             tracking_number, receiver_name, receiver_phone, receiver_address, package_weight,
             package_volume, package_items, package_count, shipping_fee, estimated_arrival,
             shipping_note, status, ship_time, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1, ?, ?, ?)"
        )
        .bind(&shipment_code)
        .bind(req.order_id)
        .bind(req.logistics_id)
        .bind(&req.logistics_name)
        .bind(&req.tracking_number)
        .bind(&receiver_info.0)  // receiver_name
        .bind(&receiver_info.1)  // receiver_phone
        .bind(&receiver_info.2)  // receiver_address
        .bind(req.package_weight)
        .bind(req.package_volume)
        .bind(&package_items_json)
        .bind(req.package_count.unwrap_or(1))
        .bind(req.shipping_fee.unwrap_or(0.0))
        .bind(&req.estimated_arrival)
        .bind(&req.shipping_note)
        .bind(&now)
        .bind(&now)
        .bind(&now)
        .execute(self.pool)
        .await?;

        let shipment_id = result.last_insert_rowid();

        // 更新订单履约状态
        self.update_order_fulfillment(req.order_id).await?;

        info!("Shipment created: {} for order {}", shipment_code, req.order_id);
        self.get_by_id(shipment_id).await.map(|o| o.unwrap())
    }

    /// 更新订单履约状态
    async fn update_order_fulfillment(&self, order_id: i64) -> sqlx::Result<()> {
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

        // 检查订单是否有多个发货单
        let (shipment_count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM shipments WHERE order_id = ?"
        )
        .bind(order_id)
        .fetch_one(self.pool)
        .await?;

        // 更新订单状态
        let fulfillment_status = if shipment_count > 1 {
            2 // 部分发货
        } else {
            3 // 已发货
        };

        sqlx::query(
            "UPDATE orders SET fulfillment_status = ?, order_status = 4, ship_time = ?, updated_at = ? WHERE id = ?"
        )
        .bind(fulfillment_status)
        .bind(&now)
        .bind(&now)
        .bind(order_id)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    /// 更新发货单
    pub async fn update(&self, id: i64, req: &UpdateShipmentRequest) -> sqlx::Result<Option<Shipment>> {
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

        sqlx::query(
            "UPDATE shipments SET
                logistics_id = COALESCE(?, logistics_id),
                logistics_name = COALESCE(?, logistics_name),
                tracking_number = COALESCE(?, tracking_number),
                package_weight = COALESCE(?, package_weight),
                package_volume = COALESCE(?, package_volume),
                shipping_fee = COALESCE(?, shipping_fee),
                actual_shipping_fee = COALESCE(?, actual_shipping_fee),
                estimated_arrival = COALESCE(?, estimated_arrival),
                actual_arrival = COALESCE(?, actual_arrival),
                status = COALESCE(?, status),
                shipping_note = COALESCE(?, shipping_note),
                updated_at = ?
             WHERE id = ?"
        )
        .bind(req.logistics_id)
        .bind(&req.logistics_name)
        .bind(&req.tracking_number)
        .bind(req.package_weight)
        .bind(req.package_volume)
        .bind(req.shipping_fee)
        .bind(req.actual_shipping_fee)
        .bind(&req.estimated_arrival)
        .bind(&req.actual_arrival)
        .bind(req.status)
        .bind(&req.shipping_note)
        .bind(&now)
        .bind(id)
        .execute(self.pool)
        .await?;

        self.get_by_id(id).await
    }

    /// 添加物流轨迹
    pub async fn add_tracking(&self, shipment_id: i64, req: &AddTrackingRequest) -> sqlx::Result<ShipmentTracking> {
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

        let result = sqlx::query(
            "INSERT INTO shipment_tracking (shipment_id, tracking_time, tracking_status,
             tracking_description, location, raw_data, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(shipment_id)
        .bind(&req.tracking_time)
        .bind(&req.tracking_status)
        .bind(&req.tracking_description)
        .bind(&req.location)
        .bind(&req.raw_data)
        .bind(&now)
        .execute(self.pool)
        .await?;

        let tracking_id = result.last_insert_rowid();

        // 如果是签收状态，更新发货单状态
        if req.tracking_status.contains("签收") || req.tracking_status.contains("delivered") {
            sqlx::query(
                "UPDATE shipments SET status = 3, actual_arrival = ?, updated_at = ? WHERE id = ?"
            )
            .bind(&req.tracking_time)
            .bind(&now)
            .bind(shipment_id)
            .execute(self.pool)
            .await?;

            // 更新订单履约状态为已签收
            let shipment = self.get_by_id(shipment_id).await?;
            if let Some(s) = shipment {
                sqlx::query(
                    "UPDATE orders SET fulfillment_status = 4, updated_at = ? WHERE id = ?"
                )
                .bind(&now)
                .bind(s.order_id)
                .execute(self.pool)
                .await?;
            }
        }

        info!("Tracking added for shipment {}", shipment_id);

        Ok(ShipmentTracking {
            id: tracking_id,
            shipment_id,
            tracking_time: req.tracking_time.clone(),
            tracking_status: req.tracking_status.clone(),
            tracking_description: req.tracking_description.clone(),
            location: req.location.clone(),
            raw_data: req.raw_data.clone(),
            created_at: now,
        })
    }

    /// 获取发货单列表
    pub async fn list(
        &self,
        page: u32,
        page_size: u32,
        query: &ShipmentQuery,
    ) -> sqlx::Result<(Vec<ShipmentListItem>, u64)> {
        let offset = (page - 1) * page_size;

        // 构建查询条件
        let mut conditions = Vec::new();
        let mut params: Vec<String> = Vec::new();

        if let Some(order_id) = query.order_id {
            conditions.push("order_id = ?");
            params.push(order_id.to_string());
        }
        if let Some(logistics_id) = query.logistics_id {
            conditions.push("logistics_id = ?");
            params.push(logistics_id.to_string());
        }
        if let Some(status) = query.status {
            conditions.push("status = ?");
            params.push(status.to_string());
        }
        if let Some(ref tracking_number) = query.tracking_number {
            conditions.push("tracking_number LIKE ?");
            params.push(format!("%{}%", tracking_number));
        }
        if let Some(ref date_from) = query.date_from {
            conditions.push("created_at >= ?");
            params.push(date_from.clone());
        }
        if let Some(ref date_to) = query.date_to {
            conditions.push("created_at <= ?");
            params.push(date_to.clone());
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        // 查询总数
        let count_sql = format!("SELECT COUNT(*) FROM shipments {}", where_clause);
        let (total,): (i64,) = if params.is_empty() {
            sqlx::query_as(&count_sql).fetch_one(self.pool).await?
        } else {
            let mut query = sqlx::query_as::<_, (i64,)>(&count_sql);
            for param in &params {
                query = query.bind(param);
            }
            query.fetch_one(self.pool).await?
        };

        // 查询列表
        let list_sql = format!(
            "SELECT id, shipment_code, order_id, logistics_name, tracking_number,
                    receiver_name, receiver_phone, status, ship_time, created_at
             FROM shipments {}
             ORDER BY id DESC LIMIT ? OFFSET ?",
            where_clause
        );

        let items: Vec<ShipmentListItem> = if params.is_empty() {
            sqlx::query_as(&list_sql)
                .bind(page_size as i32)
                .bind(offset as i32)
                .fetch_all(self.pool)
                .await?
        } else {
            let mut query = sqlx::query_as::<_, ShipmentListItem>(&list_sql);
            for param in &params {
                query = query.bind(param);
            }
            query = query.bind(page_size as i32).bind(offset as i32);
            query.fetch_all(self.pool).await?
        };

        Ok((items, total as u64))
    }
}
