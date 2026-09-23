/**
 * 在线 endpoint 封装 —— 兼容入口（Phase 8.3）。
 *
 * 实现已下沉到 `@/lib/serverApi`（无状态纯函数），使 `repositories/*`
 * 不再反向依赖 composables。此处保留同名导出，既有调用点零改动。
 *
 * 新代码请直接 `import { useServerApi, isServerNewer } from '@/lib/serverApi'`。
 */
export {
  useServerApi,
  isServerNewer,
  type ServerApi,
  type ServerCategoryLite,
  type ServerRequestLite,
  type ServerEnvironmentLite,
  type ServerEnvGroupLite,
  type ServerEnvVarLite,
} from '@/lib/serverApi'
