import { invokeUnwrap } from '@/lib/tauri'
import type { Project } from '@/types'
import { useDataModeStore } from '@/stores/dataMode'

/** 本地库返回的项目原始行（字段名与 Rust `Project` 的 camelCase 序列化输出一致）。 */
export interface RawProject {
  id: string
  name: string
  parentId: string | null
  /** 对应 Rust `sort_order`（serde camelCase → sortOrder）；此前误写为 order 导致排序恒为 0 */
  sortOrder: number
  expanded: boolean
  userId?: string
  createBy?: string
  updateBy?: string
  createTime?: string
  updateTime?: string
  currentUserRole?: string
}

/**
 * 项目数据仓库 —— 收敛项目本地库读写命令。
 *
 * 说明：项目的「在线落库」由 Rust 命令内部完成（`save_project`/`delete_project`
 * 在在线模式下自行转发后端 `/api/v1/project/*`），故此处两条路径命令相同、仅传 dataMode。
 */
export function useProjectRepo() {
  const mode = useDataModeStore()
  const modeArg = () => ({ dataMode: mode.dataMode })

  return {
    /** 读取当前模式下的项目；失败返回 null（由调用方决定是否保留现有列表）。 */
    async fetchLocal(): Promise<RawProject[] | null> {
      try {
        return (await invokeUnwrap<RawProject[]>('get_projects', { ...modeArg() })) ?? []
      } catch (e) {
        console.warn('[projectRepo] 读取项目失败', e)
        return null
      }
    },
    /** 保存项目（在线模式由 Rust 转发后端）。失败向上抛出。 */
    persist(p: Project): Promise<void> {
      const project = {
        id: p.id,
        name: p.name,
        parentId: p.parentId ?? null,
        sortOrder: p.sortOrder,
        expanded: p.expanded,
        userId: p.userId ?? '',
        createBy: p.createBy ?? '',
        updateBy: p.updateBy ?? '',
        createTime: p.createTime,
        updateTime: p.updateTime,
      }
      return invokeUnwrap<void>('save_project', { project, ...modeArg() })
    },
    update(id: string, name: string, updateBy: string) {
      return invokeUnwrap<void>('update_project', { id, name, updateBy, ...modeArg() })
    },
    remove(id: string) {
      return invokeUnwrap<void>('delete_project', { id, ...modeArg() })
    },
  }
}
