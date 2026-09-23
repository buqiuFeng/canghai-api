//! 数据模型（实体 / 枚举 / serde 形状）。
//!
//! 注意：这里的字段名与序列化形状就是跨端契约，改动需同步 TS `@/types` 与 Java 实体，
//! 并由 `scripts/contract-check.mjs` 在 CI 中校验。

use serde::{Deserialize, Deserializer, Serialize};
use validator::Validate;

/// 兼容服务端返回的 `null`：反序列化时把 null 归一为空串。
///
/// 服务端 MySQL 中的审计字段（create_by / update_by）等列允许为 NULL，而本地模型统一用
/// `String` 承载。`#[serde(default)]` 只处理「字段缺失」，不处理「显式 null」，因此
/// `null` 会直接触发 `invalid type: null, expected a string`，导致**整包同步响应解析失败**
/// （表现为点击同步即报「解析响应 JSON 失败」）。此函数用于这类字段。
pub fn null_to_empty_string<'de, D>(d: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Option::<String>::deserialize(d)?.unwrap_or_default())
}

/// 兼容服务端返回的「空串外键」：`""` 归一为 `None`（即 NULL）。
///
/// 服务端把「未分类 / 无父级 / 无分组」表示为空串（例如 `categoryId: ""`），而本地这些列
/// 带外键约束（`ch_saved_requests.category_id → ch_categories(id)`、
/// `ch_categories.parent_id → ch_categories(id)`、`ch_environments.group_id → ...`）。
/// 直接绑定空串会触发 `FOREIGN KEY constraint failed`；空串在语义上等价于「无关联」，
/// 故统一归一为 NULL，而不是伪造一个 id 为 "" 的父行。
pub fn empty_string_to_none<'de, D>(d: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Option::<String>::deserialize(d)?.filter(|s| !s.trim().is_empty()))
}

/// 兼容服务端返回的 `null`：反序列化时把 null 归一为空数组。
///
/// 服务端在「当前用户无可见项目」等场景会提前返回响应，未给列表字段赋值，
/// Jackson 默认会把 null 一并序列化下发。而 `#[serde(default)]` 只处理
/// 「字段缺失」，不处理「显式 null」，因此 null 会直接触发
/// `invalid type: null, expected a sequence`，与字符串字段的问题同源
/// （见 [`null_to_empty_string`]），同样会造成**整包同步响应解析失败**。
pub fn null_to_empty_vec<'de, D, T>(d: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Ok(Option::<Vec<T>>::deserialize(d)?.unwrap_or_default())
}

#[derive(Debug, Serialize, Deserialize, Clone, Validate)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    #[serde(default, deserialize_with = "null_to_empty_string")]
    pub user_id: String,
    /// 校验规则与 Java `ProjectSaveRequest` 的 @NotBlank 对齐
    #[validate(length(min = 1, max = 100, message = "项目名称不能为空"))]
    pub name: String,
    #[serde(default, deserialize_with = "empty_string_to_none")]
    pub parent_id: Option<String>,
    #[serde(default, alias = "order")]
    pub sort_order: i32,
    #[serde(default)]
    pub expanded: bool,
    #[serde(default, alias = "createdAt", deserialize_with = "null_to_empty_string")]
    pub create_time: String,
    #[serde(default, alias = "createdBy", deserialize_with = "null_to_empty_string")]
    pub create_by: String,
    #[serde(default, alias = "updatedAt", deserialize_with = "null_to_empty_string")]
    pub update_time: String,
    #[serde(default, alias = "updatedBy", deserialize_with = "null_to_empty_string")]
    pub update_by: String,
    #[serde(default)]
    pub deleted: bool,
    /// 当前登录用户在该项目中的角色（owner/admin/readwrite/readonly/inherit），owner 项目由 get_projects 填充
    #[serde(default, deserialize_with = "null_to_empty_string")]
    pub current_user_role: String,
}


#[derive(Debug, Serialize, Deserialize, Clone, Validate)]
#[serde(rename_all = "camelCase")]
pub struct Team {
    pub id: String,
    #[validate(length(min = 1, max = 100, message = "团队名称不能为空"))]
    pub name: String,
    #[serde(default, deserialize_with = "null_to_empty_string")]
    pub description: String,
    #[serde(default, alias = "ownerId", deserialize_with = "null_to_empty_string")]
    pub owner_id: String,
    #[serde(default, alias = "userId", deserialize_with = "null_to_empty_string")]
    pub user_id: String,
    #[serde(default, alias = "createdAt", deserialize_with = "null_to_empty_string")]
    pub create_time: String,
    #[serde(default, alias = "createdBy", deserialize_with = "null_to_empty_string")]
    pub create_by: String,
    #[serde(default, alias = "updatedAt", deserialize_with = "null_to_empty_string")]
    pub update_time: String,
    #[serde(default, alias = "updatedBy", deserialize_with = "null_to_empty_string")]
    pub update_by: String,
    #[serde(default)]
    pub deleted: bool,
}


#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TeamMember {
    pub id: String,
    #[serde(default, alias = "teamId", deserialize_with = "null_to_empty_string")]
    pub team_id: String,
    #[serde(default, alias = "userId", deserialize_with = "null_to_empty_string")]
    pub user_id: String,
    #[serde(default, deserialize_with = "null_to_empty_string")]
    pub role: String,
    #[serde(default, alias = "createdAt", deserialize_with = "null_to_empty_string")]
    pub create_time: String,
    #[serde(default, alias = "createdBy", deserialize_with = "null_to_empty_string")]
    pub create_by: String,
    #[serde(default, alias = "updatedAt", deserialize_with = "null_to_empty_string")]
    pub update_time: String,
    #[serde(default, alias = "updatedBy", deserialize_with = "null_to_empty_string")]
    pub update_by: String,
}


#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProjectMember {
    pub id: String,
    #[serde(default, alias = "projectId", deserialize_with = "null_to_empty_string")]
    pub project_id: String,
    #[serde(default, alias = "memberType", deserialize_with = "null_to_empty_string")]
    pub member_type: String,
    #[serde(default, alias = "memberId", deserialize_with = "null_to_empty_string")]
    pub member_id: String,
    /// 成员名称快照（服务端 ch_project_members.member_name：
    /// user → ch_users.username，team → ch_teams.name）。本地无用户表，离线展示依赖此列。
    #[serde(default, deserialize_with = "null_to_empty_string")]
    pub member_name: String,
    #[serde(default, deserialize_with = "null_to_empty_string")]
    pub role: String,
    #[serde(default, alias = "createdAt", deserialize_with = "null_to_empty_string")]
    pub create_time: String,
    #[serde(default, alias = "createdBy", deserialize_with = "null_to_empty_string")]
    pub create_by: String,
    #[serde(default, alias = "updatedAt", deserialize_with = "null_to_empty_string")]
    pub update_time: String,
    #[serde(default, alias = "updatedBy", deserialize_with = "null_to_empty_string")]
    pub update_by: String,
    #[serde(default)]
    pub deleted: bool,
}


#[derive(Debug, Serialize, Deserialize, Clone, Validate)]
#[serde(rename_all = "camelCase")]
pub struct SavedRequest {
    pub id: String,
    #[serde(default, deserialize_with = "null_to_empty_string")]
    pub user_id: String,
    /// 校验规则与 Java `SavedRequestService.create` 的 projectId/name 必填判定对齐
    #[validate(length(min = 1, max = 200, message = "缺少必填字段 name"))]
    pub name: String,
    #[serde(default, deserialize_with = "null_to_empty_string")]
    #[validate(length(max = 20, message = "method 非法"))]
    pub method: String,
    #[serde(default, deserialize_with = "null_to_empty_string")]
    pub url: String,
    #[serde(default)]
    pub params: serde_json::Value,
    #[serde(default)]
    pub headers: serde_json::Value,
    #[serde(default, deserialize_with = "null_to_empty_string")]
    pub body_type: String,
    #[serde(default, deserialize_with = "null_to_empty_string")]
    pub body: String,
    #[serde(default)]
    pub form_body: serde_json::Value,
    #[serde(default, deserialize_with = "empty_string_to_none")]
    pub category_id: Option<String>,
    #[serde(default, deserialize_with = "empty_string_to_none")]
    pub project_id: Option<String>,
    #[serde(default, deserialize_with = "null_to_empty_string")]
    pub pre_script: String,
    #[serde(default, deserialize_with = "null_to_empty_string")]
    pub post_script: String,
    #[serde(default)]
    pub sort_order: i32,
    #[serde(default, alias = "createdAt", deserialize_with = "null_to_empty_string")]
    pub create_time: String,
    #[serde(default, alias = "createdBy", deserialize_with = "null_to_empty_string")]
    pub create_by: String,
    #[serde(default, alias = "updatedAt", deserialize_with = "null_to_empty_string")]
    pub update_time: String,
    #[serde(default, alias = "updatedBy", deserialize_with = "null_to_empty_string")]
    pub update_by: String,
    #[serde(default)]
    pub deleted: bool,
    /// 当前登录用户在该项目中的角色（owner/admin/readwrite/readonly/inherit），owner 项目由 get_projects 填充
    #[serde(default, deserialize_with = "null_to_empty_string")]
    pub current_user_role: String,
    /// 服务端对该数据的修改时间快照（pull 自 server 的 update_time）。用于编辑冲突检测。
    #[serde(default, deserialize_with = "null_to_empty_string")]
    pub server_update_time: String,
    /// 服务端版本号快照（sync_version 列）：服务端每次变更自增（BEFORE UPDATE 触发器），
    /// 用于 update_time 相同（秒级撞车）时的冲突判定。
    /// 注意：与 ch_saved_requests 本地 `version`（请求快照版本）不是同一概念。
    #[serde(default = "default_version")]
    pub sync_version: i32,
}


/// 分类：属于某个项目（project_id），可多级嵌套（parent_id）。
/// 接口（SavedRequest）挂在分类下（category_id）。
#[derive(Debug, Serialize, Deserialize, Clone, Validate)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    pub id: String,
    #[serde(default, deserialize_with = "null_to_empty_string")]
    pub project_id: String,
    /// 校验规则与 Java `CategoryService.create` 的 projectId/name 必填判定对齐
    #[validate(length(min = 1, max = 200, message = "缺少必填字段 name"))]
    pub name: String,
    #[serde(default, deserialize_with = "empty_string_to_none")]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub sort_order: i32,
    #[serde(default)]
    pub expanded: bool,
    #[serde(default, deserialize_with = "null_to_empty_string")]
    pub create_time: String,
    #[serde(default, deserialize_with = "null_to_empty_string")]
    pub update_time: String,
    #[serde(default, deserialize_with = "null_to_empty_string")]
    pub create_by: String,
    #[serde(default, deserialize_with = "null_to_empty_string")]
    pub update_by: String,
    #[serde(default)]
    pub deleted: bool,
    /// 服务端对该数据的修改时间快照（pull 自 server 的 update_time）。用于编辑冲突检测。
    #[serde(default, deserialize_with = "null_to_empty_string")]
    pub server_update_time: String,
    /// 服务端版本号快照（sync_version 列）：服务端每次变更自增（BEFORE UPDATE 触发器），
    /// 用于 update_time 相同（秒级撞车）时的冲突判定。
    /// 注意：与 ch_saved_requests 本地 `version`（请求快照版本）不是同一概念。
    #[serde(default = "default_version")]
    pub sync_version: i32,
}


#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HistoryItem {
    pub id: String,
    #[serde(default)]
    pub method: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub params: serde_json::Value,
    #[serde(default)]
    pub headers: serde_json::Value,
    #[serde(default)]
    pub body_type: String,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub form_body: serde_json::Value,
    #[serde(default, deserialize_with = "empty_string_to_none")]
    pub category_id: Option<String>,
    /// 请求前脚本（前端 preScript）。此前结构体缺该字段，脚本落库即丢。
    #[serde(default)]
    pub pre_script: String,
    /// 请求后脚本（前端 postScript）。
    #[serde(default)]
    pub post_script: String,
    /// 所属项目（表中原有 project_id 列但结构体无字段，导致历史无法按项目隔离）。
    #[serde(default, deserialize_with = "empty_string_to_none")]
    pub project_id: Option<String>,
    #[serde(default, alias = "time")]
    pub create_time: String,
    pub status: Option<i32>,
}


#[derive(Debug, Serialize, Deserialize, Clone, Validate)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentGroup {
    pub id: String,
    #[serde(default, deserialize_with = "empty_string_to_none")]
    pub project_id: Option<String>,
    /// 校验规则与 Java `EnvironmentGroupSaveRequest` 的 @NotBlank 对齐
    #[validate(length(min = 1, max = 100, message = "缺少必填字段 name"))]
    pub name: String,
    #[serde(default)]
    pub sort_order: i32,
    #[serde(default)]
    pub expanded: bool,
    #[serde(default, alias = "createdAt", deserialize_with = "null_to_empty_string")]
    pub create_time: String,
    #[serde(default, alias = "createdBy", deserialize_with = "null_to_empty_string")]
    pub create_by: String,
    #[serde(default, alias = "updatedAt", deserialize_with = "null_to_empty_string")]
    pub update_time: String,
    #[serde(default, alias = "updatedBy", deserialize_with = "null_to_empty_string")]
    pub update_by: String,
    #[serde(default)]
    pub deleted: bool,
    /// 当前登录用户在该项目中的角色（owner/admin/readwrite/readonly/inherit），owner 项目由 get_projects 填充
    #[serde(default, deserialize_with = "null_to_empty_string")]
    pub current_user_role: String,
    /// 服务端对该数据的修改时间快照（pull 自 server 的 update_time）。用于编辑冲突检测。
    #[serde(default, deserialize_with = "null_to_empty_string")]
    pub server_update_time: String,
    /// 服务端版本号快照（sync_version 列）：服务端每次变更自增（BEFORE UPDATE 触发器），
    /// 用于 update_time 相同（秒级撞车）时的冲突判定。
    /// 注意：与 ch_saved_requests 本地 `version`（请求快照版本）不是同一概念。
    #[serde(default = "default_version")]
    pub sync_version: i32,
}


#[derive(Debug, Serialize, Deserialize, Clone, Validate)]
#[serde(rename_all = "camelCase")]
pub struct Environment {
    pub id: String,
    #[serde(default, deserialize_with = "empty_string_to_none")]
    pub project_id: Option<String>,
    /// 校验规则与 Java `EnvironmentSaveRequest` 的 @NotBlank 对齐
    #[validate(length(min = 1, max = 100, message = "缺少必填字段 name"))]
    pub name: String,
    #[serde(default, deserialize_with = "empty_string_to_none")]
    pub group_id: Option<String>,
    #[serde(default)]
    pub is_active: bool,
    #[serde(default, alias = "createdAt", deserialize_with = "null_to_empty_string")]
    pub create_time: String,
    #[serde(default, alias = "createdBy", deserialize_with = "null_to_empty_string")]
    pub create_by: String,
    #[serde(default, alias = "updatedAt", deserialize_with = "null_to_empty_string")]
    pub update_time: String,
    #[serde(default, alias = "updatedBy", deserialize_with = "null_to_empty_string")]
    pub update_by: String,
    #[serde(default)]
    pub deleted: bool,
    /// 当前登录用户在该项目中的角色（owner/admin/readwrite/readonly/inherit），owner 项目由 get_projects 填充
    #[serde(default, deserialize_with = "null_to_empty_string")]
    pub current_user_role: String,
    /// 服务端对该数据的修改时间快照（pull 自 server 的 update_time）。用于编辑冲突检测。
    #[serde(default, deserialize_with = "null_to_empty_string")]
    pub server_update_time: String,
    /// 服务端版本号快照（sync_version 列）：服务端每次变更自增（BEFORE UPDATE 触发器），
    /// 用于 update_time 相同（秒级撞车）时的冲突判定。
    /// 注意：与 ch_saved_requests 本地 `version`（请求快照版本）不是同一概念。
    #[serde(default = "default_version")]
    pub sync_version: i32,
}


#[derive(Debug, Serialize, Deserialize, Clone, Validate)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentVariable {
    pub id: String,
    /// 校验规则与 Java `EnvironmentVariableSaveRequest` 的 @NotBlank 对齐
    #[validate(length(min = 1, message = "缺少必填字段 environmentId"))]
    pub environment_id: String,
    #[validate(length(min = 1, max = 100, message = "缺少必填字段 key"))]
    pub key: String,
    #[serde(default, deserialize_with = "null_to_empty_string")]
    pub value: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub sort_order: i32,
    #[serde(default, alias = "createdAt", deserialize_with = "null_to_empty_string")]
    pub create_time: String,
    #[serde(default, alias = "createdBy", deserialize_with = "null_to_empty_string")]
    pub create_by: String,
    #[serde(default, alias = "updatedAt", deserialize_with = "null_to_empty_string")]
    pub update_time: String,
    #[serde(default, alias = "updatedBy", deserialize_with = "null_to_empty_string")]
    pub update_by: String,
    #[serde(default)]
    pub deleted: bool,
    /// 当前登录用户在该项目中的角色（owner/admin/readwrite/readonly/inherit），owner 项目由 get_projects 填充
    #[serde(default, deserialize_with = "null_to_empty_string")]
    pub current_user_role: String,
    /// 服务端对该数据的修改时间快照（pull 自 server 的 update_time）。用于编辑冲突检测。
    #[serde(default, deserialize_with = "null_to_empty_string")]
    pub server_update_time: String,
    /// 服务端版本号快照（sync_version 列）：服务端每次变更自增（BEFORE UPDATE 触发器），
    /// 用于 update_time 相同（秒级撞车）时的冲突判定。
    /// 注意：与 ch_saved_requests 本地 `version`（请求快照版本）不是同一概念。
    #[serde(default = "default_version")]
    pub sync_version: i32,
}


/// 实体默认版本号：服务端新建行为 1，本地新建（尚未上传）也按 1 处理。
fn default_version() -> i32 {
    1
}


/// 请求版本元信息（列表用，不含完整快照）。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RequestVersionMeta {
    pub id: String,
    pub version: i32,
    pub create_time: String,
    /// 是否为当前生效版本（对应 `ch_saved_requests.version`）：
    /// 前端据此标记「当前版本」并禁止回退到自身。
    #[serde(default)]
    pub is_current: bool,
}


#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProjectMemberInfo {
    pub id: String,
    pub project_id: String,
    pub member_type: String,
    pub member_id: String,
    #[serde(default, alias = "memberName")]
    pub member_name: String,
    pub role: String,
    #[serde(default, alias = "createdAt")]
    pub create_time: String,
}
