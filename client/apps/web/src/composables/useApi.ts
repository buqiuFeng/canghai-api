/**
 * 在线代理调用 —— 兼容入口（Phase 8.3）。
 *
 * 实现已下沉到 `@/lib/server`（无状态传输层，依赖 `stores/session` 与 `stores/dataMode`），
 * 使 `repositories/*` 不再反向依赖 composables。此处保留同名导出，既有调用点零改动。
 *
 * 新代码请直接 `import { invokeApi } from '@/lib/server'`。
 */
export { invokeApi, normalizeError } from '@/lib/server'
