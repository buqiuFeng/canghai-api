import { invokeUnwrap } from '@/lib/tauri'
import { useDataModeStore } from '@/stores/dataMode'
import { useServerApi } from '@/lib/serverApi'
import type { Environment, EnvironmentGroup } from '@/types'
import type { EnvironmentVariable } from '@/types'

/**
 * 环境 / 环境分组 / 环境变量 数据仓库 —— 统一封装「在线直连后端 / 离线本地库」两条持久化路径。
 *
 * composable 只负责编排（乐观更新、刷新、冲突弹窗、变量解析），
 * 全部命令名（`get_environments`/`save_environment`/…）与后端 endpoint（`/api/v1/environment/*`）收敛于此。
 */
export function useEnvironmentRepo() {
  const mode = useDataModeStore()
  const serverApi = useServerApi()
  const modeArg = () => ({ dataMode: mode.dataMode })

  /** 把后端返回的环境写入本地缓存（含 server_update_time）。 */
  async function cacheEnv(env: { id: string; projectId?: string | null; name: string; groupId?: string | null; isActive?: boolean; updateTime?: string }) {
    await invokeUnwrap<void>('save_environment', {
      environment: {
        id: env.id,
        name: env.name,
        projectId: env.projectId ?? null,
        groupId: env.groupId ?? null,
        isActive: env.isActive ?? false,
        createTime: env.updateTime ?? '',
        updateTime: env.updateTime ?? '',
        serverUpdateTime: env.updateTime ?? '',
      },
      ...modeArg(),
    })
  }

  /** 把后端返回的环境分组写入本地缓存。 */
  async function cacheGroup(group: { id: string; projectId?: string | null; name: string; sortOrder?: number; expanded?: boolean; updateTime?: string }) {
    await invokeUnwrap<void>('save_environment_group', {
      group: {
        id: group.id,
        name: group.name,
        projectId: group.projectId ?? null,
        sortOrder: group.sortOrder ?? 0,
        expanded: group.expanded ?? true,
        createTime: group.updateTime ?? '',
        updateTime: group.updateTime ?? '',
        serverUpdateTime: group.updateTime ?? '',
      },
      ...modeArg(),
    })
  }

  /** 把后端返回的环境变量写入本地缓存。 */
  async function cacheVar(v: { id: string; environmentId: string; key: string; value: string; enabled?: boolean; sortOrder?: number; updateTime?: string }) {
    await invokeUnwrap<void>('save_env_variable', {
      variable: {
        id: v.id,
        environmentId: v.environmentId,
        key: v.key,
        value: v.value,
        enabled: v.enabled ?? true,
        sortOrder: v.sortOrder ?? 0,
        createTime: v.updateTime ?? '',
        updateTime: v.updateTime ?? '',
        serverUpdateTime: v.updateTime ?? '',
      },
      ...modeArg(),
    })
  }

  return {
    isOnline: () => mode.dataMode === 'online',

    // ===== 本地读 =====
    async fetchLocal(projectId: string): Promise<{ environments: Environment[]; envGroups: EnvironmentGroup[] }> {
      try {
        const [envs, groups] = await Promise.all([
          invokeUnwrap<Environment[]>('get_environments', { projectId, ...modeArg() }),
          invokeUnwrap<EnvironmentGroup[]>('get_environment_groups', { projectId, ...modeArg() }),
        ])
        return { environments: envs ?? [], envGroups: groups ?? [] }
      } catch (e) {
        console.warn('[environmentRepo] 读取本地环境失败', e)
        return { environments: [], envGroups: [] }
      }
    },
    async fetchActiveVariables(projectId: string): Promise<EnvironmentVariable[]> {
      try {
        return (await invokeUnwrap<EnvironmentVariable[]>('get_active_env_variables', { projectId, ...modeArg() })) ?? []
      } catch (e) {
        console.warn('[environmentRepo] 读取生效环境变量失败', e)
        return []
      }
    },
    async fetchVariables(environmentId: string): Promise<EnvironmentVariable[]> {
      try {
        return (await invokeUnwrap<EnvironmentVariable[]>('get_env_variables', { environmentId, ...modeArg() })) ?? []
      } catch (e) {
        console.warn('[environmentRepo] 读取环境变量失败', e)
        return []
      }
    },

    // ===== 本地写（离线 / 缓存） =====
    saveEnvLocal(env: Environment, projectId: string) {
      return invokeUnwrap<void>('save_environment', { environment: env, projectId, ...modeArg() })
    },
    updateEnvLocal(env: Environment) {
      return invokeUnwrap<void>('update_environment', { environment: env, ...modeArg() })
    },
    deleteEnvLocal(id: string) {
      return invokeUnwrap<void>('delete_environment', { id, ...modeArg() })
    },
    activateEnvLocal(id: string, projectId: string) {
      return invokeUnwrap<void>('activate_environment', { id, projectId, ...modeArg() })
    },
    saveVarLocal(v: EnvironmentVariable) {
      return invokeUnwrap<void>('save_env_variable', { variable: v, ...modeArg() })
    },
    updateVarLocal(v: EnvironmentVariable) {
      return invokeUnwrap<void>('update_env_variable', { variable: v, ...modeArg() })
    },
    deleteVarLocal(id: string) {
      return invokeUnwrap<void>('delete_env_variable', { id, ...modeArg() })
    },
    saveGroupLocal(group: EnvironmentGroup, projectId: string) {
      return invokeUnwrap<void>('save_environment_group', { group, projectId, ...modeArg() })
    },
    updateGroupLocal(group: EnvironmentGroup) {
      return invokeUnwrap<void>('update_environment_group', { group, ...modeArg() })
    },
    deleteGroupLocal(id: string) {
      return invokeUnwrap<void>('delete_environment_group', { id, ...modeArg() })
    },
    // 折叠/展开为纯本地 UI 状态：仅写本地库，不置 dirty、不触达服务端
    setGroupExpandedLocal(id: string, expanded: boolean) {
      return invokeUnwrap<void>('set_environment_group_expanded', { id, expanded, ...modeArg() })
    },

    // ===== 在线（后端直连） =====
    listEnvironments: (projectId: string) => serverApi.listEnvironments(projectId),
    saveEnvironment: (input: Record<string, unknown>) => serverApi.saveEnvironment(input),
    updateEnvironment: (input: Record<string, unknown>) => serverApi.updateEnvironment(input),
    deleteEnvironment: (id: string) => serverApi.deleteEnvironment(id),
    listEnvGroups: (projectId: string) => serverApi.listEnvGroups(projectId),
    saveEnvGroup: (input: Record<string, unknown>) => serverApi.saveEnvGroup(input),
    updateEnvGroup: (input: Record<string, unknown>) => serverApi.updateEnvGroup(input),
    deleteEnvGroup: (id: string) => serverApi.deleteEnvGroup(id),
    saveEnvVariable: (input: Record<string, unknown>) => serverApi.saveEnvVariable(input),
    updateEnvVariable: (input: Record<string, unknown>) => serverApi.updateEnvVariable(input),
    deleteEnvVariable: (id: string) => serverApi.deleteEnvVariable(id),

    cacheEnv,
    cacheGroup,
    cacheVar,
  }
}
