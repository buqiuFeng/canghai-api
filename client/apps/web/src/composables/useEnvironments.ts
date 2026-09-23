import { computed, watch } from 'vue'
import { useProjects } from './useProjects'
import { useDataMode } from './useDataMode'
import { isServerNewer } from './useServerApi'
import { useConflict } from './useConflict'
import { uid, now } from '@/utils'
import { resolveDynamicVariable } from '@/utils/dynamicVariables'
import type { Environment, EnvironmentGroup, EnvironmentVariable } from '@/types'
import { useEnvironmentRepo } from '@/repositories/environmentRepo'
import { activeVariables, environments, envGroups, useEnvironmentsStore, variablesCache } from '@/stores/environments'

const { selectedProjectId } = useProjects()
const { dataMode } = useDataMode()
const repo = useEnvironmentRepo()
const { openConflict } = useConflict()

function isOnline() {
  return dataMode.value === 'online'
}

// 实体类型统一由 @/types 提供（Phase 6.2 #4 / 8.3），此处 re-export 兼容既有 import 路径。
export type { Environment, EnvironmentGroup }

// Phase 8.4：环境 / 分组 / 变量状态已收归 Pinia store；本 composable 只保留业务编排
const envStore = useEnvironmentsStore()

async function load() {
  try {
    const { environments: envs, envGroups: groups } = await repo.fetchLocal(selectedProjectId.value ?? '')
    envStore.replaceEnvironments(envs)
    envStore.replaceGroups(groups)
    await refreshActiveVariables()
  } catch { /* ignore */ }
}

// 切换项目时自动重新加载
watch([selectedProjectId], () => {
  load()
})

// 切换在线/离线模式时重新加载隔离数据（跳过初始触发，避免与 onMounted 重复）
watch(dataMode, () => {
  load()
}, { flush: 'post' })

async function refreshActiveVariables() {
  try {
    // 注意：后端返回的是 ApiResult 包裹结构，repo 内部已解包 .data，
    // 否则 activeVariables.value 会成为对象（无 find 方法）导致发送请求时报错
    envStore.replaceActiveVariables(await repo.fetchActiveVariables(selectedProjectId.value ?? ''))
  } catch { /* ignore */ }
}

const activeEnv = computed(() => environments.value.find(e => e.isActive) ?? null)

async function createEnv(name: string, groupId?: string | null): Promise<Environment> {
  const currentTime = now()
  const env: Environment = {
    id: uid(),
    name,
    projectId: selectedProjectId.value ?? null,
    groupId: groupId ?? null,
    isActive: false,
    createTime: currentTime,
    updateTime: currentTime,
  }
  if (isOnline()) {
    try {
      const created = await repo.saveEnvironment({
        id: env.id, // 复用本地 id，避免服务端重新生成导致 id 不一致（否则创建后无法选中）
        projectId: env.projectId ?? '',
        name,
        groupId: env.groupId ?? null,
      })
      // 用服务端真实 id 回写，保证与 load() 后的数据一致
      env.id = created.id || env.id
      env.serverUpdateTime = created.updateTime
      await repo.cacheEnv(created)
      // Phase 7.5：写操作后不再全量重拉，就地更新本地列表
      envStore.upsertEnv(env)
      // 从 store 中取真实对象返回，使上层 selectEnv 能正确命中
      return environments.value.find(e => e.id === env.id) ?? env
    } catch { /* fallthrough to local cache */ }
  }
  await repo.saveEnvLocal(env, selectedProjectId.value ?? '')
  envStore.upsertEnv(env)
  return env
}

// 在线模式：用本地完整信息提交服务器更新（含冲突检测）
async function updateEnvOnline(env: Environment): Promise<void> {
  const pid = env.projectId ?? ''
  const serverList = await repo.listEnvironments(pid)
  const server = serverList.find(s => s.id === env.id)
  if (server && isServerNewer(server.updateTime, env.serverUpdateTime, server.syncVersion, env.syncVersion)) {
    openConflict({
      title: `环境 - ${env.name}`,
      server: server as unknown as Record<string, unknown>,
      local: env as unknown as Record<string, unknown>,
      fields: [
        { key: 'name', label: '名称' },
        { key: 'groupId', label: '分组' },
      ],
      onConfirm: async () => {
        const updated = await repo.updateEnvironment({
          id: env.id,
          projectId: pid,
          name: env.name,
          groupId: env.groupId ?? null,
        })
        await repo.cacheEnv(updated)
        envStore.upsertEnv(env)
      },
    })
    return
  }
  const updated = await repo.updateEnvironment({
    id: env.id,
    projectId: pid,
    name: env.name,
    groupId: env.groupId ?? null,
  })
  await repo.cacheEnv(updated)
}

async function updateEnv(env: Environment) {
  env.updateTime = now()
  if (isOnline()) {
    try {
      await updateEnvOnline(env)
      envStore.upsertEnv(env)
      return
    } catch { /* fallthrough */ }
  }
  await repo.updateEnvLocal(env)
  envStore.upsertEnv(env)
}

async function deleteEnv(id: string) {
  if (isOnline()) {
    try {
      await repo.deleteEnvironment(id)
    } catch (e) {
      console.error('[online] 删除环境失败（已回退本地删除）:', e)
    }
  }
  await repo.deleteEnvLocal(id).catch(() => {})
  envStore.removeEnvById(id)
  if (activeEnv.value?.id === id) {
    await refreshActiveVariables()
  }
}

async function activateEnv(id: string) {
  await repo.activateEnvLocal(id, selectedProjectId.value ?? '')
  environments.value.forEach(e => e.isActive = e.id === id)
  await refreshActiveVariables()
}

async function getVariables(envId: string, force = false): Promise<EnvironmentVariable[]> {
  // 命中缓存时直接返回（除非强制刷新）；注意：强制刷新用于保存/更新变量后，
  // 否则会一直返回旧的缓存列表，导致"保存成功但列表不显示"（见 M9 修复说明）。
  if (!force && variablesCache.value[envId]) return variablesCache.value[envId]
  try {
    const vars = await repo.fetchVariables(envId)
    variablesCache.value[envId] = vars
    return vars
  } catch {
    return []
  }
}

async function saveVariable(envId: string, key: string, value: string): Promise<EnvironmentVariable> {
  const vars = variablesCache.value[envId] || []
  const maxOrder = Math.max(0, ...vars.map(v => v.sortOrder ?? 0))
  const v: EnvironmentVariable = {
    id: uid(),
    environmentId: envId,
    key,
    value,
    enabled: true,
    sortOrder: maxOrder + 1,
  }
  if (isOnline()) {
    try {
      const created = await repo.saveEnvVariable({
        environmentId: envId,
        key,
        value,
        enabled: true,
        sortOrder: v.sortOrder,
      })
      await repo.cacheVar({
        id: created.id,
        environmentId: envId,
        key: created.key,
        value: created.value,
        enabled: created.enabled,
        sortOrder: v.sortOrder,
        updateTime: created.updateTime,
      })
      // Phase 7.5：只把该变量写入缓存列表，替代 getVariables(envId, true) 的整表重拉
      envStore.upsertVariable(envId, {
        id: created.id,
        environmentId: envId,
        key: created.key,
        value: created.value,
        enabled: created.enabled ?? true,
        sortOrder: v.sortOrder,
        updateTime: created.updateTime,
      })
      if (activeEnv.value?.id === envId) await refreshActiveVariables()
      return v
    } catch { /* fallthrough */ }
  }
  await repo.saveVarLocal(v)
  envStore.upsertVariable(envId, v)
  if (activeEnv.value?.id === envId) {
    await refreshActiveVariables()
  }
  return v
}

async function updateVariable(v: EnvironmentVariable) {
  if (isOnline()) {
    try {
      const updated = await repo.updateEnvVariable({
        id: v.id,
        environmentId: v.environmentId,
        key: v.key,
        value: v.value,
        enabled: v.enabled,
        sortOrder: v.sortOrder,
      })
      await repo.cacheVar({
        id: updated.id,
        environmentId: v.environmentId,
        key: updated.key,
        value: updated.value,
        enabled: updated.enabled,
        sortOrder: v.sortOrder,
        updateTime: updated.updateTime,
      })
      // Phase 7.5：就地更新该变量，替代 getVariables(envId, true) 的整表重拉
      envStore.upsertVariable(v.environmentId, v)
      if (activeEnv.value?.id === v.environmentId) await refreshActiveVariables()
      return
    } catch { /* fallthrough */ }
  }
  await repo.updateVarLocal(v)
  envStore.upsertVariable(v.environmentId, v)
  if (activeEnv.value?.id === v.environmentId) {
    await refreshActiveVariables()
  }
}

async function deleteVariable(id: string, envId: string) {
  if (isOnline()) {
    try {
      await repo.deleteEnvVariable(id)
    } catch (e) {
      console.error('[online] 删除环境变量失败（已回退本地删除）:', e)
    }
  }
  await repo.deleteVarLocal(id).catch(() => {})
  envStore.removeVariableById(envId, id)
  if (activeEnv.value?.id === envId) {
    await refreshActiveVariables()
  }
}

/** 去除字符串中的换行/回车，防止环境变量值注入 HTTP Header（对应清单 L2） */
function stripCrlf(s: string): string {
  return String(s).replace(/[\r\n]/g, '')
}

function resolveVariables(text: string): string {
  if (!text) return text
  // 支持形如 {{key}}、{{a.b.c}} 或 Postman 动态变量 {{$guid}}、{{$timestamp}}
  return text.replace(/\{\{\s*([\w$.]+)\s*\}\}/g, (_, key: string) => {
    const v = activeVariables.value.find(x => x.enabled && x.key === key)
    if (v) {
      // L2：过滤变量值中的换行，避免 Header 注入
      return stripCrlf(v.value)
    }
    // Postman 风格动态变量（以 $ 开头）
    if (key.startsWith('$')) {
      const d = resolveDynamicVariable(key)
      if (d != null) return d
    }
    return `{{${key}}}`
  })
}

function resolveKvArray(arr: { key: string; value: string; enabled: boolean }[]): { key: string; value: string; enabled: boolean }[] {
  return arr.map(item => ({
    ...item,
    key: resolveVariables(item.key),
    value: resolveVariables(item.value),
  }))
}

// ====== 环境分组 CRUD ======

async function createGroup(name: string): Promise<EnvironmentGroup> {
  const maxOrder = Math.max(0, ...envGroups.value.map(g => g.sortOrder ?? 0))
  const currentTime = now()
  const group: EnvironmentGroup = {
    id: uid(),
    name,
    projectId: selectedProjectId.value ?? null,
    sortOrder: maxOrder + 1,
    expanded: true,
    createTime: currentTime,
    updateTime: currentTime,
  }
  if (isOnline()) {
    try {
      const created = await repo.saveEnvGroup({
        id: group.id, // 复用本地 id，避免服务端重新生成导致 id 不一致（否则后续编辑/删除失准）
        projectId: group.projectId ?? '',
        name,
        sortOrder: group.sortOrder,
      })
      // 用服务端真实 id 回写
      group.id = created.id || group.id
      group.serverUpdateTime = created.updateTime
      await repo.cacheGroup(created)
      // Phase 7.5：写操作后不再全量重拉，就地更新本地列表
      envStore.upsertGroup(group)
      // 从 store 中取真实对象返回
      return envGroups.value.find(g => g.id === group.id) ?? group
    } catch { /* fallthrough */ }
  }
  await repo.saveGroupLocal(group, selectedProjectId.value ?? '')
  envStore.upsertGroup(group)
  return group
}

// 在线模式：分组更新（含冲突检测）
async function updateGroupOnline(group: EnvironmentGroup): Promise<void> {
  const pid = group.projectId ?? ''
  const serverList = await repo.listEnvGroups(pid)
  const server = serverList.find(s => s.id === group.id)
  if (server && isServerNewer(server.updateTime, group.serverUpdateTime, server.syncVersion, group.syncVersion)) {
    openConflict({
      title: `环境分组 - ${group.name}`,
      server: server as unknown as Record<string, unknown>,
      local: group as unknown as Record<string, unknown>,
      fields: [
        { key: 'name', label: '名称' },
        { key: 'sortOrder', label: '排序' },
      ],
      onConfirm: async () => {
        const updated = await repo.updateEnvGroup({
          id: group.id,
          projectId: pid,
          name: group.name,
          sortOrder: group.sortOrder,
        })
        await repo.cacheGroup(updated)
        envStore.upsertGroup(group)
      },
    })
    return
  }
  const updated = await repo.updateEnvGroup({
    id: group.id,
    projectId: pid,
    name: group.name,
    sortOrder: group.sortOrder,
  })
  await repo.cacheGroup(updated)
}

async function updateGroup(group: EnvironmentGroup) {
  group.updateTime = now()
  if (isOnline()) {
    try {
      await updateGroupOnline(group)
      envStore.upsertGroup(group)
      return
    } catch { /* fallthrough */ }
  }
  await repo.updateGroupLocal(group)
  envStore.upsertGroup(group)
}

async function deleteGroup(id: string) {
  if (isOnline()) {
    try {
      await repo.deleteEnvGroup(id)
    } catch (e) {
      console.error('[online] 删除环境分组失败（已回退本地删除）:', e)
    }
  }
  await repo.deleteGroupLocal(id).catch(() => {})
  envStore.removeGroupById(id)
  // Set groupId to null for environments in this group
  environments.value.forEach(e => {
    if (e.groupId === id) e.groupId = null
  })
}

async function toggleGroupExpand(group: EnvironmentGroup) {
  group.expanded = !group.expanded
  envStore.upsertGroup(group)
  // 折叠/展开为纯本地 UI 状态：仅写本地库，不再触达服务端、不再重载
  await repo.setGroupExpandedLocal(group.id, group.expanded).catch(() => {})
}

async function moveEnvToGroup(envId: string, groupId: string | null) {
  const env = environments.value.find(e => e.id === envId)
  if (!env) return
  env.groupId = groupId
  env.updateTime = now()
  if (isOnline()) {
    try {
      await updateEnvOnline(env)
      envStore.upsertEnv(env)
      return
    } catch { /* fallthrough */ }
  }
  await repo.updateEnvLocal(env)
  envStore.upsertEnv(env)
}

// Phase 8.4：移除模块级 `load()` 副作用（「import 即发 RPC」使单测无法隔离）。
// 首屏由 EnvironmentManager.vue 在 onMounted 中显式调用 load()，
// 切换项目 / 数据模式仍由上方两个 watch 触达。

export function useEnvironments() {
  return {
    environments,
    envGroups,
    activeVariables,
    activeEnv,
    load,
    refreshActiveVariables,
    createEnv,
    updateEnv,
    deleteEnv,
    activateEnv,
    getVariables,
    saveVariable,
    updateVariable,
    deleteVariable,
    resolveVariables,
    resolveKvArray,
    createGroup,
    updateGroup,
    deleteGroup,
    toggleGroupExpand,
    moveEnvToGroup,
  }
}
