/**
 * 多模式数据列表的通用「重载触发器」。
 *
 * 在线 / 离线模式在物理库层面已隔离，但各实体列表（项目、分类、接口、环境等）
 * 都需要在以下时机重新拉取，且拉取时都要防止「并发请求串扰」：
 *  - dataMode 切换：用新模式物理库刷新列表；
 *  - 登录 / 登出：当前模式展示的数据应按新登录态过滤，登出后清空在线数据。
 *
 * 原先该逻辑散落在各 composable 的 watch 中（如 useProjects 的 27 行双 watch），
 * 现收敛为单一实现，避免每个实体重复一套且实现不一致。
 *
 * 去重设计：
 *   各 composable 通常在视图 onMounted 里主动调用 load() 做首次加载。
 *   useModeReload 的 watch 默认跳过初始触发（skipInitial=true），
 *   仅在 dataMode / isLoggedIn **后续变化**时才触发 reload，
 *   避免「主动 load + watch 初始触发」的双重请求。
 */
import { watch, type Ref } from 'vue'
import { useDataMode } from './useDataMode'
import { isLoggedIn } from './useSync'

const { dataMode } = useDataMode()

export interface ModeReloadOptions {
  /** 当前模式下的选中项 id（用于登出/切模式后失效校验）。传 null 表示无选中项。 */
  selectedId?: () => string | null
  /** 选中项失效时的清理动作（清除选中、回到选择页等）。 */
  onInvalidate?: () => void
  /** 是否监听登录态变化（如离线数据不依赖登录态，可关闭以减小重绘）。默认 true。 */
  watchLogin?: boolean
  /** 每次重载前执行（如登出时先清空列表，避免旧在线数据残留）。 */
  onBeforeReload?: () => void
  /**
   * 是否跳过 watch 的初始触发。默认 true。
   * 设为 false 时 watch 会在注册后立即执行一次（与 Vue watch 默认行为一致）。
   * 大多数 composable 已在视图 onMounted 中主动 load()，应保持默认 true 避免重复请求。
   */
  skipInitial?: boolean
}

/**
 * 注册数据模式 / 登录态变化时的自动重载。
 * 返回的 `modeAtCall` 供 loader 在写入结果前比对，丢弃过期响应。
 */
export function useModeReload(reload: () => Promise<void>, opts: ModeReloadOptions = {}) {
  const _reload = reload

  async function reloadAndValidate() {
    opts.onBeforeReload?.()
    await _reload()
    const id = opts.selectedId?.()
    if (id && opts.onInvalidate) opts.onInvalidate()
  }

  watch(dataMode, reloadAndValidate, { flush: 'post' })

  if (opts.watchLogin !== false) {
    watch(isLoggedIn, reloadAndValidate, { flush: 'post' })
  }
}

/** 读取「请求发起时的数据模式」，供 loader 防并发串扰。 */
export function modeAtCall(): string {
  return dataMode.value
}

/** 判断当前数据模式是否仍与发起时一致（不一致则结果已过期，应丢弃）。 */
export function modeStillValid(captured: string): boolean {
  return dataMode.value === captured
}

export type { Ref }
