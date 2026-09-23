<template>
  <div class="cat-tree">
    <div class="cat-tree-header">
      <span class="cat-tree-title">{{ t('app.category') || '接口分类' }}</span>
      <div class="cat-tree-actions">
        <el-button
          v-if="canWrite"
          size="small"
          link
          type="primary"
          :icon="Plus"
          title="添加分类"
          @click="startAdd(null)"
        />
        <el-button
          size="small"
          link
          type="primary"
          :icon="Expand"
          title="展开全部"
          @click="expandAll"
        />
        <el-button
          size="small"
          link
          type="primary"
          :icon="Fold"
          title="收起全部"
          @click="collapseAll"
        />
        <el-dropdown trigger="click" @command="handleCommand">
          <el-button size="small" link type="primary" title="更多操作">
            <el-icon><MoreFilled /></el-icon>
          </el-button>
          <template #dropdown>
            <el-dropdown-menu>
              <el-dropdown-item command="export:canghai-api">
                <el-icon><Download /></el-icon>
                导出（{{ t('app.appName') || 'CanghaiApi' }} 备份）
              </el-dropdown-item>
              <el-dropdown-item command="export:openapi">
                <el-icon><Download /></el-icon>
                导出 OpenAPI
              </el-dropdown-item>
              <el-dropdown-item command="export:postman">
                <el-icon><Download /></el-icon>
                导出 Postman
              </el-dropdown-item>
              <el-dropdown-item command="export:curl">
                <el-icon><Download /></el-icon>
                导出 cURL
              </el-dropdown-item>
              <el-dropdown-item v-if="canWrite" divided command="import">
                <el-icon><Upload /></el-icon>
                导入（OpenAPI/Postman/cURL/APIPost8）
              </el-dropdown-item>
            </el-dropdown-menu>
          </template>
        </el-dropdown>
      </div>
    </div>

    <div ref="treeScrollRef" class="cat-tree-scroll" @scroll.passive="onTreeScroll">
      <!-- 全部接口（当前项目） -->
      <div
        class="cat-node cat-node--all"
        :class="{ active: selectedCategoryId === null, 'drag-over': dragOverRoot }"
        @click="selectCategory(null)"
        @contextmenu.prevent="onRootContextMenu"
        @dragover.prevent="onRootDragOver"
        @dragleave="dragOverRoot = false"
        @drop.prevent="onRootDrop"
      >
        <el-icon><Folder /></el-icon>
        <span class="cat-name">{{ t('app.allRequests') || '全部接口' }}</span>
        <span class="cat-count">{{ projectRequests.length }}</span>
      </div>

      <!-- 根级新建分类输入框 -->
      <div v-if="addingAt === null && addingFocus" class="cat-node cat-node--edit" style="padding-left:28px">
        <el-icon><Folder /></el-icon>
        <el-input
          v-model="addName"
          size="small"
          placeholder="分类名"
          @keyup.enter="confirmAdd(null)"
          @keyup.escape="cancelAdd"
          @blur="confirmAdd(null)"
          ref="addInputRef"
        />
      </div>

      <!-- 分类树（扁平化 + 虚拟滚动）：分类行与接口行统一进入固定 30px 的虚拟列表，
           只渲染可视窗口内的行。数据多时「全部展开」也不会一次性把整棵树铺到 DOM 上。 -->
      <div :style="{ paddingTop: treeTopPad + 'px', paddingBottom: treeBottomPad + 'px' }">
        <template v-for="item in visibleFlat" :key="item.id">
          <CategoryNode
            v-if="item.kind === 'cat'"
            :flat="true"
            :node="item.node!"
            :depth="item.depth"
            :selected-id="selectedCategoryId"
            :selected-request-id="selectedRequestId ?? ''"
            :editing-id="editingId"
            :edit-name="editName"
            :adding-at="addingAt"
            :add-name="addName"
            @select="selectCategory"
            @start-add="startAdd"
            @start-add-api="(catId: string) => emit('saveNewRequest', catId)"
            @start-edit="startEdit"
            @confirm-edit="confirmEdit"
            @cancel-edit="cancelEdit"
            @delete="handleDelete"
            @toggle="toggleExpand"
            @confirm-add="confirmAdd"
            @cancel-add="cancelAdd"
            @update:add-name="addName = $event"
            @update:edit-name="editName = $event"
            @select-request="(req: SavedRequest) => { selectCategory(null); emit('selectRequest', req) }"
            @delete-request="(req: SavedRequest) => emit('deleteRequest', req)"
          />
          <div
            v-else
            class="api-node"
            :class="{ active: selectedRequestId === item.req!.id }"
            :style="{ paddingLeft: (26 + item.depth * 18) + 'px' }"
            @click="() => { selectCategory(null); emit('selectRequest', item.req!) }"
            @contextmenu.prevent="onApiRowContextMenu($event, item.req!)"
          >
            <span class="expand-spacer" />
            <el-tag size="small" :type="methodTag(item.req!.method)" effect="plain" class="api-method">
              {{ item.req!.method }}
            </el-tag>
            <span class="api-name" :title="reqUrl(item.req!)">{{ item.req!.name }}</span>
            <el-dropdown v-if="canWrite" trigger="click" @command="(cmd: string) => handleApiRowCommand(cmd, item.req!)" @click.stop class="api-dropdown">
              <span class="api-more" @click.stop>···</span>
              <template #dropdown>
                <el-dropdown-menu>
                  <el-dropdown-item command="delete">删除</el-dropdown-item>
                </el-dropdown-menu>
              </template>
            </el-dropdown>
          </div>
        </template>
      </div>
    </div>

    <!-- 右键菜单：根节点 -->
    <Teleport to="body">
      <div
        v-if="rootCtxMenu.visible && canWrite"
        class="ctx-menu"
        :style="{ left: rootCtxMenu.x + 'px', top: rootCtxMenu.y + 'px' }"
        @click.stop
      >
        <div class="ctx-menu-item" @click="rootCtxAddApi">{{ t('app.newRequest') || '添加接口' }}</div>
        <div class="ctx-menu-item" @click="rootCtxAddCategory">{{ t('app.newCategory') || '新建分类' }}</div>
      </div>
    </Teleport>

    <!-- 右键菜单：接口行 -->
    <Teleport to="body">
      <div
        v-if="apiCtxMenu.visible && canWrite"
        class="ctx-menu"
        :style="{ left: apiCtxMenu.x + 'px', top: apiCtxMenu.y + 'px' }"
        @click.stop
      >
        <div class="ctx-menu-item" @click="apiCtxEdit">{{ t('app.edit') || '编辑' }}</div>
        <div class="ctx-menu-item ctx-menu-item--danger" @click="apiCtxDelete">{{ t('app.delete') || '删除' }}</div>
      </div>
    </Teleport>

    <input ref="importInputRef" type="file" accept=".json,.yaml,.yml,.txt" style="display:none" @change="onImportFile" />
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, reactive, ref, watch } from 'vue'
import { ElMessage, ElLoading } from 'element-plus'
import { Plus, MoreFilled, Download, Upload, Folder, Expand, Fold } from '@element-plus/icons-vue'
import CategoryNode from './CategoryNode.vue'
import { useCategories } from '@/composables/useCategories'
import { useVirtualWindow } from '@/composables/useVirtualWindow'
import { useSavedRequests } from '@/composables/useSavedRequests'
import { useTeams } from '@/composables/useTeams'
import { useDataMode } from '@/composables/useDataMode'
import { sync } from '@/composables/useSync'
import type { ImportSummary } from '@/lib/server'
import { invokeUnwrap } from '@/lib/tauri'
import { i18n } from '@/i18n'
import type { SavedRequest } from '@/composables/useSavedRequests'
import type { Category } from '@/composables/useCategories'

const t = i18n.global.t
const props = defineProps<{
  projectId?: string | null
  selectedRequestId?: string
}>()

const emit = defineEmits<{
  (e: 'selectRequest', req: SavedRequest): void
  (e: 'deleteRequest', req: SavedRequest): void
  (e: 'saveNewRequest', categoryId: string): void
}>()

const { canWrite } = useTeams()
const { dataMode } = useDataMode()
const {
  categories,
  selectedCategoryId,
  load: loadCategories,
  selectCategory,
  getChildren,
  addCategory,
  updateCategory,
  deleteCategory,
  toggleExpand,
  updateExpanded,
} = useCategories()
const { savedRequests, load: loadRequests, getRequestsByCategory } = useSavedRequests()

// 项目切换时加载分类树与接口
watch(
  () => props.projectId,
  (pid) => {
    if (pid) {
      loadCategories(pid)
      loadRequests()
    }
  },
  { immediate: true },
)

const rootNodes = computed(() => getChildren(null))
const projectRequests = computed(() =>
  props.projectId ? savedRequests.value.filter(r => r.projectId === props.projectId) : savedRequests.value,
)

// ====== 分类树扁平化 + 虚拟滚动（数据多时全部展开也不卡） ======
// 关键：虚拟滚动要求「每行固定高度」。因此把「当前展开状态」下可见的
// 分类节点【及其接口行】一并拍平成一维数组，每行均为固定 30px。
// 接口列表不再由各 CategoryNode 内部渲染（那是可变高度，会破坏滚动定位），
// 而是作为独立行进入同一个虚拟列表——分类数与接口数两处都被虚拟化了。
type FlatKind = 'cat' | 'api'
interface FlatNode { kind: FlatKind; id: string; node?: Category; req?: SavedRequest; depth: number }
const flatNodes = computed<FlatNode[]>(() => {
  const out: FlatNode[] = []
  const walk = (cats: Category[], depth: number) => {
    for (const c of cats) {
      out.push({ kind: 'cat', id: c.id, node: c, depth })
      if (c.expanded) {
        // 先子分类（与原始渲染顺序一致），再本分类下的接口行
        walk(getChildren(c.id), depth + 1)
        for (const r of getRequestsByCategory(c.id)) {
          out.push({ kind: 'api', id: r.id, req: r, depth: depth + 1 })
        }
      }
    }
  }
  walk(rootNodes.value, 0)
  return out
})

const TREE_ROW_HEIGHT = 30
const treeScrollRef = ref<HTMLElement | null>(null)
const {
  start: treeStart,
  end: treeEnd,
  topPad: treeTopPad,
  bottomPad: treeBottomPad,
  onScroll: onTreeScroll,
  syncViewport: syncTreeViewport,
} = useVirtualWindow({ total: () => flatNodes.value.length, itemHeight: TREE_ROW_HEIGHT })

const visibleFlat = computed(() => flatNodes.value.slice(treeStart.value, treeEnd.value))

// 容器尺寸变化（或可见节点总数变化）时重新测量视口，否则只会渲染 overscan 行
watch([flatNodes, () => props.projectId], async () => {
  await nextTick()
  syncTreeViewport(treeScrollRef.value)
})
onMounted(() => syncTreeViewport(treeScrollRef.value))

function setExpandAll(expanded: boolean) {
  for (const c of categories) {
    if ((c.expanded ?? false) !== expanded) updateExpanded(c.id, expanded)
  }
}
function expandAll() { setExpandAll(true) }
function collapseAll() { setExpandAll(false) }

// 接口行的小工具（与 CategoryNode 保持一致的展示）
function methodTag(m: string): 'success' | 'primary' | 'warning' | 'danger' | 'info' {
  switch (m) {
    case 'GET': return 'success'
    case 'POST': return 'primary'
    case 'PUT': return 'warning'
    case 'PATCH': return 'warning'
    case 'DELETE': return 'danger'
    default: return 'info'
  }
}
function reqUrl(req: SavedRequest): string {
  return req.url || req.name || '未命名接口'
}

// ====== 内联编辑/新增 ======
const editingId = ref<string | null>(null)
const editName = ref('')
const addingAt = ref<string | null>(null)
const addName = ref('')
const addingFocus = ref(false)
const addInputRef = ref<any>(null)
const dragOverRoot = ref(false)
const rootCtxMenu = ref<{ visible: boolean; x: number; y: number }>({ visible: false, x: 0, y: 0 })

// 接口行右键菜单（扁平虚拟列表中接口行由本组件直接渲染，需在本地维护菜单状态）
const apiCtxMenu = reactive<{ visible: boolean; x: number; y: number; req: SavedRequest | null }>({
  visible: false, x: 0, y: 0, req: null,
})
function onApiRowContextMenu(e: MouseEvent, req: SavedRequest) {
  apiCtxMenu.visible = true
  apiCtxMenu.x = e.clientX
  apiCtxMenu.y = e.clientY
  apiCtxMenu.req = req
}
function apiCtxEdit() {
  if (apiCtxMenu.req) emit('selectRequest', apiCtxMenu.req)
  apiCtxMenu.visible = false
}
function apiCtxDelete() {
  if (apiCtxMenu.req) emit('deleteRequest', apiCtxMenu.req)
  apiCtxMenu.visible = false
}
function handleApiRowCommand(cmd: string, req: SavedRequest) {
  if (cmd === 'delete') emit('deleteRequest', req)
}

function startAdd(parentId: string | null) {
  editingId.value = null
  addingAt.value = parentId
  addName.value = ''
  addingFocus.value = true
  nextTick(() => {
    if (parentId === null) addInputRef.value?.focus?.()
  })
}

function confirmAdd(parentId: string | null) {
  const name = addName.value.trim()
  addingFocus.value = false
  addingAt.value = null
  if (!name) return
  if (props.projectId) {
    const cat = addCategory(name, parentId, props.projectId)
    if (cat) {
      selectCategory(cat.id)
      cat.expanded = true
    }
  }
  addName.value = ''
}

function cancelAdd() {
  addingAt.value = null
  addingFocus.value = false
  addName.value = ''
}

function startEdit(id: string) {
  addingAt.value = null
  editingId.value = id
  const cat = categories.find(c => c.id === id)
  editName.value = cat ? cat.name : ''
}

function confirmEdit() {
  const id = editingId.value
  const name = editName.value.trim()
  editingId.value = null
  if (id && name) updateCategory(id, name)
}

function cancelEdit() {
  editingId.value = null
}

function handleDelete(id: string) {
  deleteCategory(id)
}

// ====== 右键菜单 ======
function onRootContextMenu(e: MouseEvent) {
  rootCtxMenu.value = { visible: true, x: e.clientX, y: e.clientY }
}
function rootCtxAddApi() {
  rootCtxMenu.value.visible = false
  emit('saveNewRequest', '')
}
function rootCtxAddCategory() {
  rootCtxMenu.value.visible = false
  startAdd(null)
}
function onRootDragOver() { dragOverRoot.value = true }
function onRootDrop() { dragOverRoot.value = false }

// ====== 顶部下拉：导出 / 导入 ======
const {
  exportRequests,
  exportOpenApi,
  exportPostman,
  exportCurlForAll,
  downloadJson,
  downloadText,
  parseImportFile,
  readImportFileText,
  importRequests,
} = useSavedRequests()

function currentCategoryTree(): Category[] {
  return props.projectId
    ? categories.filter(c => c.projectId === props.projectId)
    : categories
}

function handleCommand(cmd: string) {
  if (cmd === 'export:canghaiApi') {
    downloadJson(exportRequests(currentCategoryTree()), `canghai-api-backup-${Date.now()}.json`)
  } else if (cmd === 'export:openapi') {
    downloadText(exportOpenApi(currentCategoryTree()), `openapi-${Date.now()}.json`, 'application/json')
  } else if (cmd === 'export:postman') {
    downloadText(exportPostman(currentCategoryTree()), `postman-${Date.now()}.json`, 'application/json')
  } else if (cmd === 'export:curl') {
    downloadText(exportCurlForAll(currentCategoryTree()), `requests-${Date.now()}.sh`, 'text/plain')
  } else if (cmd === 'import') {
    triggerImport()
  }
}

// 导入
const importInputRef = ref<HTMLInputElement | null>(null)
function triggerImport() {
  importInputRef.value?.click()
}
async function onImportFile(e: Event) {
  const input = e.target as HTMLInputElement
  const file = input.files?.[0]
  input.value = ''
  if (!file) return
  if (!props.projectId) {
    ElMessage.warning('请先选择项目再导入')
    return
  }
  const projectId = props.projectId
  // 导入可能耗时较长（在线模式还要服务端解析落库 + 同步，后端超时 180s），
  // 全程加一个遮罩，避免用户重复点击或以为卡死。
  const loading = ElLoading.service({
    lock: true,
    text: '正在导入数据，请稍候…',
    background: 'rgba(0, 0, 0, 0.45)',
  })
  try {
    if (dataMode.value === 'online') {
      // 在线模式：走 Rust `import_collection` 统一解析 + 落本地（dirty=1）+ 推云端（增量同步）。
      // 该命令已覆盖 OpenAPI/Postman/cURL/APIPost8/CanghaiApi 等格式，
      // 并会把 APIPost 的 apt.* 脚本转换为本软件支持的 pm.* 语法（见 commands/import.rs）。
      const text = await readImportFileText(file)
      const r = await invokeUnwrap<ImportSummary>('import_collection', {
        fileName: file.name,
        content: text,
        projectId,
        dataMode: 'online',
      })
      // 导入已写入本地并推送到云端；再同步一次刷新本地缓存（失败不影响导入结果）。
      try {
        await sync()
      } catch (e) {
        console.warn('[import] 导入成功，但同步失败（本地列表可能稍后才刷新）:', e)
      }
      // 分类与接口从本地库重读，确保导入的数据立即出现在列表中。
      loadCategories(projectId)
      loadRequests()
      const envTip = r.envImported ? `，环境 ${r.envImported}（变量 ${r.envVarImported}）` : ''
      ElMessage.success(`导入完成：成功 ${r.imported}${envTip}`)
      return
    }
    // 离线模式：无后端可上传，保留前端本地解析（与 useImporters.ts 为同一套格式支持）
    const data = await parseImportFile(file)
    const { imported, skipped, envImported, envVarImported } = await importRequests(data, (name, parentId) =>
      addCategory(name, parentId, projectId),
      projectId,
    )
    loadCategories(projectId)
    const envTip = envImported ? `，环境 ${envImported}（变量 ${envVarImported}）` : ''
    ElMessage.success(`导入完成：成功 ${imported}，跳过 ${skipped}${envTip}`)
  } catch (err) {
    ElMessage.error('导入失败：' + ((err as Error).message || String(err)))
  } finally {
    loading.close()
  }
}

// 点击空白处关闭右键菜单
if (typeof window !== 'undefined') {
  window.addEventListener('click', () => {
    rootCtxMenu.value.visible = false
    apiCtxMenu.visible = false
  })
}
</script>

<style scoped>
.cat-tree { height: 100%; }
.cat-tree-scroll {
  max-height: calc(100vh - 200px);
  overflow-y: auto;
  overflow-x: hidden;
}
/* 虚拟滚动要求行高固定，否则上下占位高度与真实滚动位置不匹配 */
.cat-tree-scroll :deep(.cat-node) {
  height: 30px;
  box-sizing: border-box;
  padding-top: 0;
  padding-bottom: 0;
}

/* 接口行样式：已从 CategoryNode 移入本组件（拍平后接口行由 CategoryTree 直接渲染，
   不再继承 CategoryNode 的 scoped 样式），这里补回布局与交互样式。 */
.api-node {
  display: flex;
  align-items: center;
  gap: 6px;
  height: 30px;
  box-sizing: border-box;
  padding-top: 0;
  padding-bottom: 0;
  padding-right: 6px;
  border-radius: 7px;
  cursor: pointer;
  font-size: var(--fs-sm);
  color: var(--text-2, #334155);
  transition: all 0.12s cubic-bezier(0.16, 1, 0.3, 1);
  user-select: none;
}
.api-node:hover { background: rgba(99,102,241,0.04); }
.api-node.active {
  background: linear-gradient(135deg, rgba(99,102,241,.08), rgba(139,92,246,.04));
  box-shadow: 0 1px 2px rgba(99,102,241,0.04);
}
.api-method { flex-shrink: 0; font-size: var(--fs-2xs); border-radius: 4px !important; }
.api-name { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-weight: 500; }
.api-dropdown { display: none; flex-shrink: 0; }
.api-node:hover .api-dropdown { display: inline-flex; }
.api-more {
  font-size: var(--fs-lg); color: var(--text-4, #94a3b8); cursor: pointer;
  padding: 0 2px; line-height: 1; letter-spacing: -1px; border-radius: 3px;
}
.api-more:hover { color: var(--text-2, #334155); background: rgba(99,102,241,0.08); }
.expand-spacer { width: 14px; height: 14px; flex-shrink: 0; }
.cat-tree-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 10px;
  border-bottom: 1px solid var(--border, #ebeef5);
}
.cat-tree-title { font-weight: 600; font-size: var(--fs-md); color: var(--text, #303133); }
.cat-tree-actions { display: flex; align-items: center; gap: 4px; }

.cat-node {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 5px 10px;
  cursor: pointer;
  font-size: var(--fs-md);
  color: var(--text, #303133);
  border-radius: 4px;
}
.cat-node:hover { background: var(--hover, #f5f7fa); }
.cat-node.active { background: var(--primary-light, #ecf5ff); color: var(--primary, #409eff); }
.cat-node--all { font-weight: 600; }
.cat-name { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.cat-count { font-size: var(--fs-xs); color: var(--text-secondary, #909399); }
.cat-node--edit { gap: 4px; }

.ctx-menu {
  position: fixed;
  z-index: 3000;
  background: var(--surface);
  border: 1px solid var(--border, #ebeef5);
  border-radius: 6px;
  box-shadow: 0 2px 12px rgba(0,0,0,0.12);
  padding: 4px 0;
  min-width: 140px;
}
.ctx-menu-item {
  padding: 7px 14px;
  font-size: var(--fs-md);
  cursor: pointer;
  color: var(--text, #303133);
}
.ctx-menu-item:hover { background: var(--hover, #f5f7fa); }
</style>
