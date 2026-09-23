import { createPinia, setActivePinia } from 'pinia'

/**
 * 全局 Pinia 实例。
 *
 * 本项目的 composable 长期采用「模块级单例」共享状态（useDataMode/useSettings/useConflict 等），
 * 这些状态在**模块顶层**（组件外）即被访问。为了让 Pinia store 也能在组件外使用，
 * 这里创建实例后立即 `setActivePinia`，保证任何位置调用 `useXxxStore()` 都可用。
 * `main.ts` 中仍需 `app.use(pinia)` 完成向应用安装。
 */
export const pinia = createPinia()
setActivePinia(pinia)
