import { invoke } from '@tauri-apps/api/core'
import { invokeUnwrap } from '@/lib/tauri'
import { useDataModeStore } from '@/stores/dataMode'
import { useServerApi } from '@/lib/serverApi'
import type { SavedRequest } from '@/types'

/** 接口历史版本元信息。 */
export interface RequestVersionMeta {
  id: string
  version: number
  create_time: string
  /** 是否为当前生效版本；当前版本不可回退到自身 */
  is_current: boolean
}

/**
 * 接口（saved request）数据仓库 —— 统一封装「在线直连后端 / 离线本地库」两条持久化路径。
 *
 * composable 只负责编排（乐观更新、刷新、冲突弹窗、导入导出），命令名与后端 endpoint 收敛于此。
 */
export function useRequestRepo() {
  const mode = useDataModeStore()
  const serverApi = useServerApi()
  const modeArg = () => ({ dataMode: mode.dataMode })

  /** 把服务端返回的接口（或本地原始 req）缓存到本地存储。
   *  注意：必须透传全部字段（params/headers/body/脚本等），否则刷新后这些字段会丢失。 */
  async function cacheFromServer(req: Record<string, any>, projectId?: string | null) {
    await invoke('save_saved_request', {
      request: {
        id: req.id,
        projectId: projectId ?? req.projectId ?? null,
        name: req.name,
        method: req.method ?? 'GET',
        url: req.url ?? '',
        categoryId: req.categoryId ?? null,
        sortOrder: typeof req.sortOrder === 'number' ? req.sortOrder : 0,
        params: Array.isArray(req.params) ? req.params : [],
        headers: Array.isArray(req.headers) ? req.headers : [],
        bodyType: req.bodyType ?? 'none',
        body: req.body ?? '',
        formBody: Array.isArray(req.formBody) ? req.formBody : [],
        preScript: req.preScript ?? '',
        postScript: req.postScript ?? '',
        updateTime: req.updateTime ?? '',
        serverUpdateTime: req.updateTime ?? '',
        // 服务端版本号作为冲突判定的本地基线；缺失时不传（Rust 侧默认 1），
        // 前端 isServerNewer 在本地无版本基线时也不会做版本兜底
        syncVersion: typeof req.syncVersion === 'number' ? req.syncVersion : undefined,
      },
      ...modeArg(),
    })
  }

  return {
    isOnline: () => mode.dataMode === 'online',

    // ===== 本地库（离线 / 缓存） =====
    async fetchLocal(projectId: string): Promise<SavedRequest[]> {
      try {
        return (
          (await invokeUnwrap<SavedRequest[]>('get_saved_requests', {
            projectId,
            ...modeArg(),
          })) ?? []
        )
      } catch (e) {
        // 兜底：本地读取失败时退化为空列表（保持原静默行为，但不再手写信封判断）
        console.warn('[requestRepo] 读取本地接口列表失败', e)
        return []
      }
    },
    persistLocal(request: SavedRequest) {
      return invoke('save_saved_request', { request, ...modeArg() })
    },
    updateLocal(request: SavedRequest) {
      return invoke('update_saved_request', { request, ...modeArg() })
    },
    deleteLocal(id: string) {
      return invoke('delete_saved_request', { id, ...modeArg() })
    },

    // ===== 在线（后端直连） =====
    listServer: (projectId: string) => serverApi.listRequests(projectId),
    createServer: (input: Record<string, unknown>) => serverApi.createRequest(input),
    updateServer: (input: Record<string, unknown>) => serverApi.updateRequest(input),
    deleteServer: (id: string) => serverApi.deleteRequest(id),

    // ===== 历史版本 =====
    // 注意：这两个命令在 Rust 侧同样返回 `ApiResult<T>` 信封
    // （见 `commands/saved_request.rs`），必须经 `invokeUnwrap` 拆包。
    // 此前误用裸 `invoke`，listVersions 拿到的是整个信封对象（`versions.length` 为
    // undefined → 抽屉恒判为空），restoreVersion 的失败也不会 reject。
    listVersions(requestId: string): Promise<RequestVersionMeta[]> {
      return invokeUnwrap<RequestVersionMeta[]>('list_request_versions', { requestId, ...modeArg() })
    },
    /** 读取指定版本的完整内容（版本对比用） */
    getVersionSnapshot(requestId: string, version: number): Promise<SavedRequest> {
      return invokeUnwrap<SavedRequest>('get_request_version_snapshot', { requestId, version, ...modeArg() })
    },
    restoreVersion(requestId: string, version: number): Promise<SavedRequest> {
      return invokeUnwrap<SavedRequest>('restore_request_version', { requestId, version, ...modeArg() })
    },

    cacheFromServer,
  }
}
