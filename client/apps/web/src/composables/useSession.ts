/**
 * 会话 / 认证共享态 —— 兼容入口（Phase 8.3 / 8.4）。
 *
 * 状态已下沉到 Pinia `@/stores/session`（叶子 store）。此处保留同名导出
 * （`syncStatus` / `authToken` / `currentTeamId` / `isLoggedIn` …），
 * 使 `useSync` / `useTeams` / `useApi` 等既有调用点的 `xxx.value` 写法零改动。
 *
 * 新代码请直接 `import { useSessionStore } from '@/stores/session'`。
 */
export {
  useSessionStore,
  syncStatus,
  syncMessage,
  syncOnline,
  currentTeamId,
  authToken,
  syncServerUrl,
  syncConfigLoaded,
  currentUser,
  isLoggedIn,
} from '@/stores/session'
