/**
 * 在线模式直连后端 HTTP 接口的封装（Phase 8.3 由 `composables/useServerApi.ts` 下沉）。
 *
 * 改造背景：
 * - 同步改为「只从服务器拉取」（pull-only），本地库作为服务器数据的只读缓存 + 离线兜底。
 * - 在线模式下，分类 / 接口 / 环境变量的新增 / 编辑 / 删除操作，直接调用后端 HTTP 接口，
 *   由后端落库并返回最新 `updateTime`。
 * - 编辑冲突检测：编辑前先从服务器拉取该条最新数据，如果服务器的 `updateTime` 比本地
 *   缓存的 `serverUpdateTime` 新，则弹「数据对比框」让用户确认后再保存。
 *
 * 这里只做 HTTP 传输层与冲突检测辅助，具体的本地缓存刷新仍由各 composable / store 负责。
 *
 * 下沉理由：本模块是**无状态纯函数集合**，此前位于 composables 下导致
 * `repositories/* → composables/*` 反向依赖；现在依赖方向统一为 `repositories → lib → stores`。
 * `composables/useServerApi.ts` 仍按原名 re-export，既有调用点零改动。
 */

import { invokeApi } from './server'
import { parseServerTime } from '@/utils'

/** 分类：后端返回结构（仅取冲突检测需要的字段）。 */
export interface ServerCategoryLite {
  id: string
  name: string
  parentId?: string | null
  icon?: string
  sortOrder?: number
  updateTime?: string
  /** 服务端版本号（V2 起后端下发），用于秒级时间戳相同的并发判定 */
  syncVersion?: number
}

/** 接口：后端返回结构（仅取冲突检测需要的字段）。 */
export interface ServerRequestLite {
  id: string
  name: string
  method?: string
  url?: string
  updateTime?: string
  syncVersion?: number
}

/** 环境：后端返回结构。 */
export interface ServerEnvironmentLite {
  id: string
  name: string
  updateTime?: string
  syncVersion?: number
}

/** 环境分组：后端返回结构。 */
export interface ServerEnvGroupLite {
  id: string
  name: string
  updateTime?: string
  syncVersion?: number
}

/** 环境变量：后端返回结构。 */
export interface ServerEnvVarLite {
  id: string
  key: string
  value: string
  enabled?: boolean
  updateTime?: string
  syncVersion?: number
}

/** 把后端 updateTime 转成可比较的时间戳（空串视为 0）。 */
function toTs(t?: string): number {
  if (!t) return 0
  // 统一走 parseServerTime（按 +08:00 解读存储值），避免这里按本机时区裸解析
  // 导致与服务端比较时整体偏一个时区
  const n = parseServerTime(t)
  return Number.isNaN(n) ? 0 : n
}

/**
 * 冲突检测：比较服务器最新更新时间与本地缓存的 serverUpdateTime。
 * 返回 true 表示「服务器更新，存在冲突」。
 *
 * Phase 4：updateTime 只有秒级精度，同一秒内被他人修改时无法区分，
 * 因此当时间相同（或都缺失）时用服务端版本号 syncVersion 兜底比较。
 *
 * ⚠️ 两个「不作判定」的前置条件（修复误报冲突）：
 * 1) 本地没有服务端时间基线时（`serverUpdateTime` 为空，例如本地新建、或写接口未回传
 *    updateTime），**不代表**「服务器更新了」——按不冲突处理，交由服务端按 update_time 决定；
 * 2) 本地没有版本基线时不启用版本兜底：部分写接口（环境 / 分组 / 变量的 update）只返回
 *    字符串、**不回传 syncVersion**，本地恒为 undefined，若按「有值 > 无值」比较，
 *    服务端每次自增都会让「自己刚保存过的行」再更新时误弹冲突框。
 */
export function isServerNewer(
  serverUpdateTime?: string,
  localServerUpdateTime?: string,
  serverSyncVersion?: number,
  localSyncVersion?: number,
): boolean {
  const localTs = toTs(localServerUpdateTime)
  // 无本地基线：无法判断是否被他人改过，不制造冲突
  if (!localTs) return false
  const diff = toTs(serverUpdateTime) - localTs
  if (diff !== 0) return diff > 0
  // 秒级撞车：仅当本地确有版本基线时才用版本号兜底
  if (typeof localSyncVersion !== 'number') return false
  return (serverSyncVersion ?? 0) > localSyncVersion
}

export function useServerApi() {
  return {
    // ===== 分类 =====
    listCategories: (projectId: string) =>
      invokeApi<ServerCategoryLite[]>('/api/v1/category/list', { projectId }),
    createCategory: (cat: Record<string, unknown>) =>
      invokeApi<ServerCategoryLite>('/api/v1/category/create', cat),
    updateCategory: (cat: Record<string, unknown>) =>
      invokeApi<ServerCategoryLite>('/api/v1/category/update', cat),
    deleteCategory: (id: string) => invokeApi<void>('/api/v1/category/delete', { id }),

    // ===== 接口（saved request） =====
    listRequests: (projectId: string) =>
      invokeApi<ServerRequestLite[]>('/api/v1/request/list', { projectId }),
    createRequest: (req: Record<string, unknown>) =>
      invokeApi<ServerRequestLite>('/api/v1/request/create', req),
    updateRequest: (req: Record<string, unknown>) =>
      invokeApi<ServerRequestLite>('/api/v1/request/update', req),
    deleteRequest: (id: string) => invokeApi<void>('/api/v1/request/delete', { id }),

    // ===== 环境变量（环境 / 分组 / 变量） =====
    listEnvironments: (projectId: string) =>
      invokeApi<ServerEnvironmentLite[]>('/api/v1/environment/list', { projectId }),
    saveEnvironment: (env: Record<string, unknown>) =>
      invokeApi<ServerEnvironmentLite>('/api/v1/environment/save', env),
    updateEnvironment: (env: Record<string, unknown>) =>
      invokeApi<ServerEnvironmentLite>('/api/v1/environment/update', env),
    deleteEnvironment: (id: string) => invokeApi<void>('/api/v1/environment/delete', { id }),

    listEnvGroups: (projectId: string) =>
      invokeApi<ServerEnvGroupLite[]>('/api/v1/environment/groups', { projectId }),
    saveEnvGroup: (group: Record<string, unknown>) =>
      invokeApi<ServerEnvGroupLite>('/api/v1/environment/groups/save', group),
    updateEnvGroup: (group: Record<string, unknown>) =>
      invokeApi<ServerEnvGroupLite>('/api/v1/environment/groups/update', group),
    deleteEnvGroup: (id: string) => invokeApi<void>('/api/v1/environment/groups/delete', { id }),

    listEnvVariables: (environmentId: string) =>
      invokeApi<ServerEnvVarLite[]>('/api/v1/environment/variables/list', { environmentId }),
    saveEnvVariable: (variable: Record<string, unknown>) =>
      invokeApi<ServerEnvVarLite>('/api/v1/environment/variables/save', variable),
    updateEnvVariable: (variable: Record<string, unknown>) =>
      invokeApi<ServerEnvVarLite>('/api/v1/environment/variables/update', variable),
    deleteEnvVariable: (id: string) => invokeApi<void>('/api/v1/environment/variables/delete', { id }),
  }
}

export type ServerApi = ReturnType<typeof useServerApi>
