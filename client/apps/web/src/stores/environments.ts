import { ref } from 'vue'
import { defineStore, storeToRefs } from 'pinia'
import '@/stores/pinia'
import type { Environment, EnvironmentGroup, EnvironmentVariable } from '@/types'

/**
 * 环境 / 分组 / 变量状态（Phase 8.4：状态收归 Pinia；Phase 7.5：乐观更新原语）。
 *
 * 与 `stores/savedRequests` 相同：只持有状态与最小变更原语，业务编排（在线-离线分支、
 * 冲突检测、本地库/后端双写）仍留在 `composables/useEnvironments.ts`。
 *
 * `upsertVariable` 直接操作按环境 id 分组的缓存，替代原先「保存一个变量就
 * `getVariables(envId, true)` 重拉整个列表」的写法。
 */
export const useEnvironmentsStore = defineStore('environments', () => {
  const environments = ref<Environment[]>([])
  const envGroups = ref<EnvironmentGroup[]>([])
  const activeVariables = ref<EnvironmentVariable[]>([])
  const variablesCache = ref<Record<string, EnvironmentVariable[]>>({})

  /** 全量替换（仅用于 load 拉取本地库结果） */
  function replaceEnvironments(list: Environment[]) {
    environments.value = list
  }

  /** 全量替换分组 */
  function replaceGroups(list: EnvironmentGroup[]) {
    envGroups.value = list
  }

  /** 替换「当前激活环境的变量」（用于刷新变量解析上下文） */
  function replaceActiveVariables(list: EnvironmentVariable[]) {
    activeVariables.value = list
  }

  function upsertEnv(item: Environment) {
    const next = { ...item }
    const idx = environments.value.findIndex(e => e.id === item.id)
    if (idx === -1) environments.value.push(next)
    else environments.value[idx] = next
  }

  function removeEnvById(id: string) {
    const idx = environments.value.findIndex(e => e.id === id)
    if (idx !== -1) environments.value.splice(idx, 1)
    delete variablesCache.value[id]
  }

  function upsertGroup(item: EnvironmentGroup) {
    const next = { ...item }
    const idx = envGroups.value.findIndex(g => g.id === item.id)
    if (idx === -1) envGroups.value.push(next)
    else envGroups.value[idx] = next
  }

  function removeGroupById(id: string) {
    const idx = envGroups.value.findIndex(g => g.id === id)
    if (idx !== -1) envGroups.value.splice(idx, 1)
  }

  /** 写入某环境的变量列表（插入或就地更新），保持缓存与列表一致 */
  function upsertVariable(envId: string, item: EnvironmentVariable) {
    if (!variablesCache.value[envId]) variablesCache.value[envId] = []
    const next = { ...item }
    const idx = variablesCache.value[envId].findIndex(v => v.id === item.id)
    if (idx === -1) variablesCache.value[envId].push(next)
    else variablesCache.value[envId][idx] = next
  }

  function removeVariableById(envId: string, id: string) {
    const list = variablesCache.value[envId]
    if (!list) return
    const idx = list.findIndex(v => v.id === id)
    if (idx !== -1) list.splice(idx, 1)
  }

  return {
    environments,
    envGroups,
    activeVariables,
    variablesCache,
    replaceEnvironments,
    replaceGroups,
    replaceActiveVariables,
    upsertEnv,
    removeEnvById,
    upsertGroup,
    removeGroupById,
    upsertVariable,
    removeVariableById,
  }
})

/** 兼容导出：既有调用点按 `xxx.value` 读写，语义与迁移前的模块级 ref 一致。 */
export const { environments, envGroups, activeVariables, variablesCache } =
  storeToRefs(useEnvironmentsStore())
