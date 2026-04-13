// 货币格式化工具函数

/// 根据货币类型格式化价格
pub fn format_price(amount: f64, currency: &str) -> String {
    match currency.to_uppercase().as_str() {
        "USD" => format!("${:.2}", amount),
        "CNY" => format!("¥{:.2}", amount),
        "EUR" => format!("€{:.2}", amount),
        "GBP" => format!("£{:.2}", amount),
        _ => format!("{:.2}", amount),
    }
}

/// 格式化 USD 价格
pub fn format_usd(amount: f64) -> String {
    format!("${:.2}", amount)
}

/// 格式化 CNY 价格
pub fn format_cny(amount: f64) -> String {
    format!("¥{:.2}", amount)
}

/// 格式化金额（带货币符号）
pub fn format_amount(amount: f64, currency: Option<&str>) -> String {
    match currency {
        Some(c) => format_price(amount, c),
        None => format!("{:.2}", amount),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_usd() {
        assert_eq!(format_usd(99.0), "$99.00");
        assert_eq!(format_usd(1234.56), "$1234.56");
    }

    #[test]
    fn test_format_cny() {
        assert_eq!(format_cny(99.0), "¥99.00");
        assert_eq!(format_cny(1234.56), "¥1234.56");
    }

    #[test]
    fn test_format_price() {
        assert_eq!(format_price(99.0, "USD"), "$99.00");
        assert_eq!(format_price(99.0, "CNY"), "¥99.00");
        assert_eq!(format_price(99.0, "EUR"), "€99.00");
    }
}
