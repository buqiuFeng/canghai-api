import { invoke } from '@tauri-apps/api/core'
import { invokeUnwrap } from '@/lib/tauri'
import { useDataModeStore } from '@/stores/dataMode'
import { useServerApi } from '@/lib/serverApi'

/** 本地库中的分类原始行（Rust 返回 camelCase）。 */
export interface RawCategory {
  id: string
  projectId: string
  name: string
  parentId: string | null
  sortOrder: number
  expanded: boolean
}

/** 需要写本地库的分类字段（含 server_update_time 快照）。 */
export interface LocalCategoryWrite {
  id: string
  projectId: string | null
  name: string
  parentId: string | null
  sortOrder: number
  expanded: boolean
  serverUpdateTime?: string
}

/** 服务端返回的分类精简结构（仅取缓存所需字段）。 */
interface ServerCategoryLike {
  id: string
  projectId?: string | null
  name: string
  parentId?: string | null
  sortOrder?: number
  expanded?: boolean
  updateTime?: string
}

/**
 * 分类数据仓库 —— 统一封装「在线直连后端 / 离线本地库」两条持久化路径。
 *
 * 设计目标（Phase 2 repository 层试点）：
 * - 命令名（`get_categories`/`save_category`/…）与后端 endpoint（`/api/v1/category/*`）不再散落在 composable；
 * - composable 只负责编排（乐观更新、刷新）与冲突弹窗，不再感知数据来源。
 * 后续 resource（request/environment/…）按同一模式搬迁。
 */
export function useCategoryRepo() {
  const mode = useDataModeStore()
  const serverApi = useServerApi()
  const modeArg = () => ({ dataMode: mode.dataMode })

  /** 把后端返回的分类写入本地库作为缓存（含 server_update_time）。 */
  async function cacheFromServer(cat: ServerCategoryLike, projectId?: string | null) {
    await invoke('save_category', {
      category: {
        id: cat.id,
        name: cat.name,
        parentId: cat.parentId ?? null,
        sortOrder: cat.sortOrder ?? 0,
        expanded: cat.expanded ?? true,
        // 后端 ServerCategoryLite 不含 projectId，由调用方补上，
        // 否则本地存库 project_id 为 null，get_categories 按 project_id 过滤时查不到。
        projectId: projectId ?? cat.projectId ?? null,
        createTime: cat.updateTime ?? '',
        updateTime: cat.updateTime ?? '',
        serverUpdateTime: cat.updateTime ?? '',
      },
      ...modeArg(),
    })
  }

  return {
    isOnline: () => mode.dataMode === 'online',

    // ===== 本地库（离线 / 缓存） =====
    async fetchLocal(projectId: string): Promise<RawCategory[]> {
      try {
        return (
          (await invokeUnwrap<RawCategory[]>('get_categories', {
            projectId,
            ...modeArg(),
          })) ?? []
        )
      } catch (e) {
        // 兜底：本地读取失败时退化为空列表（保持原静默行为，但不再手写信封判断）
        console.warn('[categoryRepo] 读取本地分类失败', e)
        return []
      }
    },
    persistLocal(cat: LocalCategoryWrite) {
      return invoke('save_category', {
        category: {
          id: cat.id,
          projectId: cat.projectId,
          name: cat.name,
          parentId: cat.parentId ?? null,
          sortOrder: cat.sortOrder,
          expanded: cat.expanded,
          serverUpdateTime: cat.serverUpdateTime,
        },
        ...modeArg(),
      })
    },
    updateLocal(id: string, name: string) {
      return invoke('update_category', { id, name, ...modeArg() })
    },
    deleteLocal(id: string) {
      return invoke('delete_category', { id, ...modeArg() })
    },
    setExpandedLocal(id: string, expanded: boolean) {
      return invoke('set_category_expanded', { id, expanded, ...modeArg() })
    },

    // ===== 在线（后端直连 + 本地缓存） =====
    listServer: (projectId: string) => serverApi.listCategories(projectId),
    async createServer(input: Record<string, unknown>, projectId: string) {
      const created = await serverApi.createCategory(input)
      await cacheFromServer(created as ServerCategoryLike, projectId)
      return created
    },
    async updateServer(input: Record<string, unknown>, projectId?: string | null) {
      const updated = await serverApi.updateCategory(input)
      await cacheFromServer(updated as ServerCategoryLike, projectId)
      return updated
    },
    deleteServer: (id: string) => serverApi.deleteCategory(id),
    cacheFromServer,
  }
}
