/// 格式化数值：整数不带小数点，浮点保留原样。
fn fmt_num(v: f64) -> String {
    if v == v.trunc() && v.abs() < 1e9 {
        format!("{}", v as i64)
    } else {
        format!("{}", v)
    }
}

/// 格式化 time / duration：四舍五入到 i32。
fn fmt_time(v: f64) -> String {
    format!("{}", v.round() as i32)
}

