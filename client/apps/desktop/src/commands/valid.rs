//! 命令层参数校验（Phase 6.6）。
//!
//! 与 Java 侧的 `@NotBlank` + `GlobalExceptionHandler` **同语义**：
//! 校验失败统一返回 `PARAM_INVALID(40000)`，使前端在在线 / 离线两种数据模式下
//! 拿到同一套错误码与提示口径，不再依赖「命令内零散 if 判断 + 各写各的文案」。
//!
//! 用法：
//! ```ignore
//! use crate::commands::valid;
//!
//! #[tauri::command]
//! pub fn save_category(category: Category, ...) -> Result<ApiResult<()>, String> {
//!     if let Err(e) = valid::check(&category) { return Ok(e); }
//!     if let Err(e) = valid::required("projectId", &category.project_id) { return Ok(e); }
//!     ...
//! }
//! ```
use crate::sync::{codes, ApiResult};
use validator::Validate;

/// 结构体级声明式校验（规则写在 `db.rs` 的结构体上，此处只负责收敛错误码与文案）。
pub fn check<T: Validate>(value: &T) -> Result<(), ApiResult<()>> {
    value.validate().map_err(|errs| {
        ApiResult::err(codes::PARAM_INVALID, format!("参数校验失败：{errs}"))
    })
}

/// 标量必填校验：`id` / `environmentId` 这类结构性参数为空时继续执行必然失败
/// （要么写坏数据、要么抛 DB 错误），因此在命令入口直接拦下。
pub fn required(field: &str, value: &str) -> Result<(), ApiResult<()>> {
    if value.trim().is_empty() {
        return Err(ApiResult::err(
            codes::PARAM_INVALID,
            format!("缺少 {field}"),
        ));
    }
    Ok(())
}

/// 把校验错误转成任意 data 类型的失败信封：
/// 供返回裸 `ApiResult<T>` 的同步命令使用（`Result<ApiResult<T>, String>` 的命令直接用 `Ok(e)`）。
pub fn as_fail<T>(e: ApiResult<()>) -> ApiResult<T> {
    ApiResult::err(e.code, e.msg)
}
