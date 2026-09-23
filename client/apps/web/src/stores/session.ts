import { computed, ref } from 'vue'
import { defineStore, storeToRefs } from 'pinia'
import '@/stores/pinia'
import type { UserInfo } from '@/types'

/**
 * 会话 / 认证共享态（Phase 8.3 / 8.4）。
 *
 * 定位：依赖图中的**叶子 store** —— 只依赖 `pinia` 与 `@/types`，
 * 不 import 任何 composable / repository / lib，因此可作为共享态的最终归属，打断：
 *
 * - `useSync ↔ useApi`：`useApi` 需要 `authToken`，而 `useSync` 需要 `normalizeError`；
 * - `useTeams ↔ useSync`：`useTeams` 需要 `currentUser/currentTeamId`，
 *   而 `useSync` 需要 `teams/loadTeams/loadCurrentTeamMembers`；
 * - `repositories/* ↔ composables/*`：各 repository 原先从 composable 取 `currentTeamId`。
 *
 * 兼容策略：模块底部用 `storeToRefs` 导出**同名 ref**，`composables/useSession.ts`
 * 原样 re-export，因此既有调用点（`authToken.value` 等）零改动。
 */
export const useSessionStore = defineStore('session', () => {
  /** 同步状态 */
  const syncStatus = ref<'idle' | 'syncing' | 'success' | 'error'>('idle')
  /** 同步提示文案 */
  const syncMessage = ref('')
  /** 上次同步是否在线完成（用于界面提示） */
  const syncOnline = ref(false)
  /**
   * 当前团队 ID（不可切换）。
   * 团队维度不再支持用户切换：始终使用登录用户所属团队（优先登录返回 teamId，
   * 否则回退到持久化 SyncConfig 中的 team_id，离线环境使用默认团队）。
   * 该值仅作为后端命令的 team_id 参数，用于数据归属/过滤，不参与前端路由或视图切换。
   */
  const currentTeamId = ref<string>('default')
  /** 认证 token（Tauri 模式下由 keyring 支撑，Web 降级模式下加密存 localStorage） */
  const authToken = ref('')
  /**
   * 持久化配置里的 serverUrl（启动期由 `initAuth` 读 `get_sync_config` 时发布）。
   *
   * 用途：settings store 与 AppLayout 顶栏的同步配置**复用这一次读取**，
   * 避免刷新页面时对同一条 `get_sync_config` 命令发起多次 IPC。
   */
  const syncServerUrl = ref('')
  /**
   * 启动期是否已成功读取过持久化同步配置。
   * 供 `loadSyncConfig` 判断「是否还需要补读一次」——读取成功但未登录时无需再补读。
   */
  const syncConfigLoaded = ref(false)
  /** 当前登录用户 */
  const currentUser = ref<UserInfo | null>(null)
  /** 是否已登录 */
  const isLoggedIn = computed(() => !!authToken.value && !!currentUser.value)

  return {
    syncStatus,
    syncMessage,
    syncOnline,
    currentTeamId,
    authToken,
    syncServerUrl,
    syncConfigLoaded,
    currentUser,
    isLoggedIn,
  }
})

// 兼容导出：mutable ref（可读可写），与迁移前的模块级 ref 语义完全一致。
export const {
  syncStatus,
  syncMessage,
  syncOnline,
  currentTeamId,
  authToken,
  syncServerUrl,
  syncConfigLoaded,
  currentUser,
  isLoggedIn,
} = storeToRefs(useSessionStore())
