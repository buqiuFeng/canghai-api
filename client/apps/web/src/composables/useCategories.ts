import { reactive, ref } from 'vue'
import { useDataMode } from './useDataMode'
import type { Category } from '@/types'
import { isServerNewer } from './useServerApi'
import { useConflict } from './useConflict'
import { useCategoryRepo, type RawCategory } from '@/repositories/categoryRepo'

export type { Category }

const { dataMode } = useDataMode()
const repo = useCategoryRepo()
const { openConflict } = useConflict()

function isOnline() {
  return dataMode.value === 'online'
}

function uid(): string {
  return 'c_' + Math.random().toString(36).slice(2, 10) + Date.now().toString(36)
}

// 分类：属于某个项目（projectId），可多级嵌套（parentId）
const categories = reactive<Category[]>([])
const selectedCategoryId = ref<string | null>(null)

function mapRow(r: RawCategory): Category {
  return {
    id: r.id,
    projectId: r.projectId,
    name: r.name,
    parentId: r.parentId ?? null,
    sortOrder: r.sortOrder ?? 0,
    expanded: r.expanded ?? false,
    serverUpdateTime: (r as RawCategory & { serverUpdateTime?: string }).serverUpdateTime,
  }
}

// 加载指定项目下的分类树（本地库读；在线写入后亦刷新本地缓存）
async function load(projectId: string) {
  try {
    const rows = await repo.fetchLocal(projectId)
    categories.splice(0, categories.length, ...rows.map(mapRow))
  } catch { /* ignore */ }
}

function selectCategory(id: string | null) {
  selectedCategoryId.value = id
}

// 取某父级下的直接子分类（parentId === parentId）
function getChildren(parentId: string | null): Category[] {
  return categories.filter(c => (c.parentId ?? null) === (parentId ?? null))
    .sort((a, b) => a.sortOrder - b.sortOrder || a.name.localeCompare(b.name))
}

function findCategory(id: string): Category | undefined {
  return categories.find(c => c.id === id)
}

// 新建分类：可挂在项目根级（parentId=null）或某分类下（多级）
function addCategory(name: string, parentId: string | null = null, projectId?: string): Category | null {
  if (!projectId) return null
  const siblings = getChildren(parentId)
  const maxOrder = Math.max(0, ...siblings.map(c => c.sortOrder))
  const cat: Category = {
    id: uid(),
    projectId,
    name,
    parentId: parentId ?? null,
    sortOrder: maxOrder + 1,
    expanded: true,
  }
  if (isOnline()) {
    repo.createServer({
      projectId,
      name,
      parentId: cat.parentId,
      sortOrder: cat.sortOrder,
    }, projectId).then(() => load(projectId)).catch((e) => {
      console.error('[online] 新建分类失败:', e)
    })
    return cat
  }
  categories.push(cat)
  if (parentId) {
    const parent = findCategory(parentId)
    if (parent) parent.expanded = true
  }
  persist(cat)
  return cat
}

// 在线模式：用本地完整信息提交服务器更新（含冲突检测）
async function updateCategoryOnline(id: string, patch: Partial<Category>, projectId: string): Promise<boolean> {
  const serverList = await repo.listServer(projectId)
  const server = serverList.find(s => s.id === id)
  const local = findCategory(id)
  const payload = {
    id,
    projectId,
    name: patch.name ?? local?.name ?? '',
    parentId: patch.parentId ?? local?.parentId ?? null,
    sortOrder: patch.sortOrder ?? local?.sortOrder ?? 0,
  }
  if (server && isServerNewer(server.updateTime, local?.serverUpdateTime, server.syncVersion, local?.syncVersion)) {
    openConflict({
      title: `分类 - ${local?.name ?? ''}`,
      server: server as unknown as Record<string, unknown>,
      local: { ...local, ...patch } as unknown as Record<string, unknown>,
      fields: [
        { key: 'name', label: '名称' },
        { key: 'parentId', label: '父分类' },
        { key: 'sortOrder', label: '排序' },
      ],
      onConfirm: async () => {
        await repo.updateServer(payload, projectId)
        await load(projectId)
      },
    })
    return false
  }
  await repo.updateServer(payload, projectId)
  return true
}

function updateCategory(id: string, name: string) {
  const cat = findCategory(id)
  if (cat) cat.name = name
  if (isOnline()) {
    const projectId = cat?.projectId ?? ''
    updateCategoryOnline(id, { name }, projectId).then(() => load(projectId)).catch((e) => {
      console.error('[online] 更新分类失败:', e)
    })
    return
  }
  repo.updateLocal(id, name).catch(() => {})
}

function deleteCategory(id: string) {
  // 所属项目必须在下面的「乐观移除」之前取到：节点被 splice 掉之后
  // findCategory(id) 必然返回 undefined，projectId 退化成 ''，
  // 而 load('') 会拿空 projectId 去查本地库（命中 0 行）→ 整棵分类树被清空。
  // 这就是「删掉一个分类后分类树全都没了」的直接原因（数据仍在本地库，重新 load 即恢复）。
  const projectId = findCategory(id)?.projectId ?? ''

  const removeIds = new Set<string>()
  function collect(pid: string) {
    removeIds.add(pid)
    categories.filter(c => (c.parentId ?? null) === pid).forEach(c => collect(c.id))
  }
  collect(id)
  categories.splice(0, categories.length, ...categories.filter(c => !removeIds.has(c.id)))
  if (removeIds.has(selectedCategoryId.value ?? '')) {
    selectedCategoryId.value = null
  }
  if (isOnline()) {
    // 服务端删除成功后必须同步清本地缓存行：本地库还留着该分类时，
    // 紧接着的 load(projectId) 会把它原样读回来（表现为「删了又出现」，
    // 要等下一次 pull 拿到删除墓碑才消失）。与接口删除（useSavedRequests.deleteRequest）
    // 的「先删服务端、再删本地」两步式保持一致。
    repo.deleteServer(id)
      .then(() => repo.deleteLocal(id))
      .then(() => load(projectId))
      .catch((e) => {
        console.error('[online] 删除分类失败:', e)
      })
    return
  }
  repo.deleteLocal(id).catch(() => {})
}

function toggleExpand(id: string) {
  const cat = findCategory(id)
  if (cat) {
    cat.expanded = !cat.expanded
    updateExpanded(id, cat.expanded)
  }
}

function updateExpanded(id: string, expanded: boolean) {
  const cat = findCategory(id)
  if (cat) cat.expanded = expanded
  // 折叠/展开为纯本地 UI 状态：仅写本地库，不再同步到服务端（避免触发 call_server_api / 重载）。
  repo.setExpandedLocal(id, expanded).catch(() => {})
}

function persist(cat: Category) {
  repo.persistLocal({
    id: cat.id,
    projectId: cat.projectId,
    name: cat.name,
    parentId: cat.parentId ?? null,
    sortOrder: cat.sortOrder,
    expanded: cat.expanded,
    serverUpdateTime: cat.serverUpdateTime,
  }).catch(() => {})
}

export function useCategories() {
  return {
    categories,
    selectedCategoryId,
    load,
    selectCategory,
    getChildren,
    findCategory,
    addCategory,
    updateCategory,
    deleteCategory,
    toggleExpand,
    updateExpanded,
  }
}
