import { invokeUnwrap } from '@/lib/tauri'
import type { ProjectMemberInfo } from '@/types'
import { useDataModeStore } from '@/stores/dataMode'

/**
 * 项目成员数据仓库 —— 收敛项目成员的本地库/后端命令。
 * 成员管理在离线模式下由 Rust 侧拒绝，前端仍通过同一命令入口。
 */
export function useProjectMemberRepo() {
  const mode = useDataModeStore()
  const modeArg = () => ({ dataMode: mode.dataMode })

  return {
    async fetchLocal(projectId: string): Promise<ProjectMemberInfo[]> {
      try {
        return (
          (await invokeUnwrap<ProjectMemberInfo[]>('get_project_members', {
            projectId,
            ...modeArg(),
          })) ?? []
        )
      } catch (e) {
        console.warn('[projectMemberRepo] 读取项目成员失败', e)
        return []
      }
    },
    add(projectId: string, memberType: 'user' | 'team', memberId: string, role: ProjectMemberInfo['role']) {
      return invokeUnwrap<void>('add_project_member', {
        projectId,
        memberType,
        memberId,
        role,
        ...modeArg(),
      })
    },
    remove(projectId: string, memberType: 'user' | 'team', memberId: string) {
      return invokeUnwrap<void>('remove_project_member', { projectId, memberType, memberId, ...modeArg() })
    },
  }
}
