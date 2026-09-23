import { ref, computed, nextTick, watch } from 'vue'
import type { TabState, ResponseInfo, ScriptLogEntry } from '@/types'
import { emptyKV } from '@/types'

const TABS_STORAGE_KEY = 'canghai.api.debugger.tabs'

/** 仅持久化表单与 tab 元数据，剥离响应体等运行时字段；同时记录页签归属的项目 */
function serializeTabs(list: TabState[], activeId: string, projectId: string): string {
  const slim = list.map(t => ({
    id: t.id,
    title: t.title,
    form: t.form,
    reqTab: t.reqTab,
    currentRequestId: t.currentRequestId,
  }))
  return JSON.stringify({ projectId, tabs: slim, activeTabId: activeId })
}

function loadPersistedTabs(projectId: string): { tabs: TabState[]; activeTabId: string } {
  try {
    const raw = localStorage.getItem(TABS_STORAGE_KEY)
    if (!raw) return { tabs: [], activeTabId: '' }
    const parsed = JSON.parse(raw) as { projectId?: string; tabs?: any[]; activeTabId?: string }
    // 页签归属项目：持久化的页签不属于当前项目时直接丢弃，等价于「切换项目即关闭」。
    // 否则点「切换」离开项目 A 再进入项目 B 时，A 打开的接口页签会被原样带进 B。
    if ((parsed.projectId ?? '') !== projectId) {
      localStorage.removeItem(TABS_STORAGE_KEY)
      return { tabs: [], activeTabId: '' }
    }
    const tabs: TabState[] = (parsed.tabs ?? []).map((t: any) => {
      const base = createDefaultTabState(t.title)
      return {
        ...base,
        id: t.id ?? base.id,
        title: t.title ?? base.title,
        form: {
          ...base.form,
          ...(t.form ?? {}),
          params: Array.isArray(t.form?.params) && t.form.params.length ? t.form.params : [emptyKV()],
          headers: Array.isArray(t.form?.headers) && t.form.headers.length ? t.form.headers : [emptyKV()],
          formBody: Array.isArray(t.form?.formBody) && t.form.formBody.length ? t.form.formBody : [emptyKV()],
        },
        reqTab: t.reqTab ?? 'params',
        currentRequestId: t.currentRequestId ?? '',
      }
    })
    return { tabs, activeTabId: parsed.activeTabId ?? (tabs[0]?.id ?? '') }
  } catch {
    return { tabs: [], activeTabId: '' }
  }
}

/** 创建默认页签状态 */
export function createDefaultTabState(title?: string): TabState {
  return {
    id: `tab-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`,
    title: title || '新请求',
    form: {
      method: 'GET',
      url: '',
      params: [emptyKV()],
      headers: [emptyKV()],
      bodyType: 'none',
      body: '',
      formBody: [emptyKV()],
      categoryId: undefined,
      preScript: '',
      postScript: '',
    },
    reqTab: 'params',
    respTab: 'body',
    respView: 'pretty',
    response: null,
    requestHeaders: [],
    error: '',
    loading: false,
    currentRequestId: '',
    scriptLog: [],
  }
}

/**
 * 页签状态。
 *
 * @param getProjectId 返回当前项目 id。页签按项目隔离：读取到的持久化页签若不属于该项目，
 *                     直接丢弃（等价于「切换项目后关闭之前打开的接口页签」）。
 *                     用 getter 而非值，是为了在同一组件实例内切换路由参数（换项目）后，
 *                     落盘记录的仍是**最新**项目 id，否则刷新会因 id 过期而丢掉页签。
 */
export function useTabs(getProjectId: () => string = () => '') {
  // 初始不创建默认页签：首次进入展示系统说明页，用户「开始调试 / 新建」后再创建。
  // 若本地有上次刷新前遗留的页签（已打开的接口）**且属于同一个项目**，则恢复它们，
  // 避免刷新丢失正在编辑的内容；换了项目则视为已关闭，不再恢复。
  const persisted = loadPersistedTabs(getProjectId())
  const tabs = ref<TabState[]>(persisted.tabs)
  const activeTabId = ref(persisted.activeTabId)
  const editingTabId = ref<string>('')
  const editingTabTitle = ref('')
  const tabInputRef = ref<HTMLInputElement[] | null>(null)
  const tabListRef = ref<HTMLElement | null>(null)

  /** 将当前 tabs 状态写入 localStorage，刷新后可恢复 */
  function persistTabs() {
    try {
      localStorage.setItem(TABS_STORAGE_KEY, serializeTabs(tabs.value, activeTabId.value, getProjectId()))
    } catch {
      /* 忽略写入失败（如隐私模式） */
    }
  }

  /**
   * 内容变更的防抖持久化（Phase 7.7）。
   *
   * 原实现 `watch([tabs, activeTabId], ..., { deep: true })` 会在**每次按键**触发：
   * 深度遍历整个 tab 状态（含大响应体）+ JSON.stringify + 同步写 localStorage，
   * 直接体现为输入延迟。这里对内容变更做 500ms 防抖，结构性变更（新增/关闭/切换）
   * 仍然立即落盘，保证不会丢失页签。
   */
  const PERSIST_DEBOUNCE_MS = 500
  let persistTimer: ReturnType<typeof setTimeout> | null = null

  function schedulePersist() {
    if (persistTimer !== null) clearTimeout(persistTimer)
    persistTimer = setTimeout(() => {
      persistTimer = null
      persistTabs()
    }, PERSIST_DEBOUNCE_MS)
  }

  /** 立即落盘（并取消挂起的防抖任务），用于结构性变更 */
  function persistNow() {
    if (persistTimer !== null) {
      clearTimeout(persistTimer)
      persistTimer = null
    }
    persistTabs()
  }

  // 结构性变更（页签数量 / 活跃页签）立即持久化
  watch([() => tabs.value.length, activeTabId], () => persistNow())
  // 内容变更（表单输入、页签重命名等）防抖持久化，避免每次按键都写 localStorage
  watch(tabs, () => schedulePersist(), { deep: true })

  /**
   * 快照当前活跃 tab 的响应式状态。
   * 调用方需要传入当前表单/tabs 的响应式值的 getter。
   */
  function snapshotCurrentTab(getters: {
    form: TabState['form']
    reqTab: TabState['reqTab']
    respTab: TabState['respTab']
    respView: TabState['respView']
    response: ResponseInfo | null
    requestHeaders: { key: string; value: string }[]
    error: string
    loading: boolean
    currentRequestId: string
    scriptLog: ScriptLogEntry[]
  }): TabState {
    return {
      id: activeTabId.value,
      title: tabs.value.find(t => t.id === activeTabId.value)?.title ?? '',
      form: {
        method: getters.form.method,
        url: getters.form.url,
        params: JSON.parse(JSON.stringify(getters.form.params)),
        headers: JSON.parse(JSON.stringify(getters.form.headers)),
        bodyType: getters.form.bodyType,
        body: getters.form.body,
        formBody: JSON.parse(JSON.stringify(getters.form.formBody)),
        categoryId: getters.form.categoryId,
        preScript: getters.form.preScript,
        postScript: getters.form.postScript,
      },
      reqTab: getters.reqTab,
      respTab: getters.respTab,
      respView: getters.respView,
      response: getters.response,
      requestHeaders: [...getters.requestHeaders],
      error: getters.error,
      loading: getters.loading,
      currentRequestId: getters.currentRequestId,
      scriptLog: [...getters.scriptLog],
    }
  }

  function saveCurrentTabState(getters: Parameters<typeof snapshotCurrentTab>[0]) {
    const idx = tabs.value.findIndex(t => t.id === activeTabId.value)
    if (idx >= 0) {
      tabs.value[idx] = snapshotCurrentTab(getters)
    }
  }

  function startEditTabTitle(tabId: string) {
    editingTabId.value = tabId
    const tab = tabs.value.find(t => t.id === tabId)
    editingTabTitle.value = tab?.title ?? ''
    nextTick(() => {
      const inputs = tabInputRef.value
      if (inputs && inputs.length > 0) {
        inputs[0].focus()
        inputs[0].select()
      }
    })
  }

  function finishEditTabTitle() {
    if (!editingTabId.value) return
    const tab = tabs.value.find(t => t.id === editingTabId.value)
    if (tab) {
      tab.title = editingTabTitle.value.trim()
    }
    editingTabId.value = ''
  }

  function cancelEditTabTitle() {
    editingTabId.value = ''
  }

  /**
   * 关闭全部页签，回到「系统说明页」空状态。
   *
   * 用于切换项目：页签归属项目，上一个项目打开的接口不应留在下一个项目里。
   * 清空后由上面的结构性 watch（页签数量变化）立即落盘，保证刷新/重进也不会恢复。
   */
  function closeAllTabs() {
    tabs.value = []
    activeTabId.value = ''
    editingTabId.value = ''
    editingTabTitle.value = ''
  }

  const currentTabTitle = computed({
    get: () => {
      const t = tabs.value.find(t => t.id === activeTabId.value)
      return t?.title ?? ''
    },
    set: (val: string) => {
      const t = tabs.value.find(t => t.id === activeTabId.value)
      if (t) t.title = val
    },
  })

  return {
    tabs,
    activeTabId,
    editingTabId,
    editingTabTitle,
    tabInputRef,
    tabListRef,
    currentTabTitle,
    snapshotCurrentTab,
    saveCurrentTabState,
    persistTabs,
    startEditTabTitle,
    finishEditTabTitle,
    cancelEditTabTitle,
    closeAllTabs,
  }
}
