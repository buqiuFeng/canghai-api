import { computed, ref } from 'vue'

/**
 * 窗口化（虚拟）列表的纯计算逻辑（Phase 7.6）。
 *
 * 为什么不用 `vue-virtual-scroller` 之类的依赖：本项目只需「固定行高的定长窗口」，
 * 引入第三方库会增加打包体积与版本维护面；这里 40 行纯函数即可覆盖，
 * 且不依赖 DOM 结构（只有 `onScroll` 读一次滚动容器），可以直接单测。
 *
 * 用法（配合 padding 占位，保持单一处行标记）：
 * ```vue
 * <div ref="listRef" @scroll.passive="onScroll"
 *      :style="{ paddingTop: topPad + 'px', paddingBottom: bottomPad + 'px' }">
 *   <div v-for="item in items.slice(start, end)" :key="item.id">…</div>
 * </div>
 * ```
 */
export interface VirtualWindowOptions {
  /** 列表总条数（传函数以保持响应式） */
  total: () => number
  /** 单行固定高度（px），必须与 CSS 中该行的实际高度一致 */
  itemHeight: number
  /** 视口上下各多渲染的行数，避免快速滚动时出现空白 */
  overscan?: number
}

export function useVirtualWindow(options: VirtualWindowOptions) {
  const scrollTop = ref(0)
  const viewportHeight = ref(0)
  const overscan = options.overscan ?? 4

  const start = computed(() =>
    Math.max(0, Math.floor(scrollTop.value / options.itemHeight) - overscan),
  )

  const end = computed(() => {
    const visible = Math.ceil((scrollTop.value + viewportHeight.value) / options.itemHeight) + overscan
    // 至少渲染 start 之后的一屏，且不超过总数
    return Math.min(options.total(), Math.max(start.value, visible))
  })

  /** 顶部占位高度：让被裁剪掉的行仍占据滚动空间 */
  const topPad = computed(() => start.value * options.itemHeight)

  /** 底部占位高度 */
  const bottomPad = computed(() =>
    Math.max(0, (options.total() - end.value) * options.itemHeight),
  )

  function onScroll(e: Event) {
    const el = e.target as HTMLElement | null
    if (!el) return
    scrollTop.value = el.scrollTop
    viewportHeight.value = el.clientHeight
  }

  /** 首次进入虚拟模式（容器刚渲染）时测量视口高度，否则只会渲染 overscan 行 */
  function syncViewport(el: HTMLElement | null | undefined) {
    if (!el) return
    viewportHeight.value = el.clientHeight
  }

  function reset() {
    scrollTop.value = 0
  }

  return { start, end, topPad, bottomPad, onScroll, syncViewport, reset }
}
