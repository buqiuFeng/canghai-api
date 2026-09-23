<template>
  <div>
    <!-- 分类节点 -->
    <div
      class="cat-node"
      :class="{ active: selectedId === node.id, 'drag-over': dragOverId === node.id, 'dragging': draggingId === node.id }"
      :style="{ paddingLeft: (12 + depth * 18) + 'px' }"
      @click="$emit('select', node.id)"
      @contextmenu.prevent="onCatContextMenu($event, node)"
    >
      <el-icon
        v-if="children.length || requests.length"
        class="expand-icon"
        :class="{ expanded: node.expanded }"
        @click.stop="$emit('toggle', node.id)"
      >
        <ArrowRight />
      </el-icon>
      <span v-else class="expand-spacer" />

      <el-icon class="folder-icon"><Folder /></el-icon>

      <template v-if="editingId === node.id">
        <el-input
          :model-value="editName"
          size="small"
          class="edit-input"
          @update:model-value="$emit('update:editName', $event)"
          @keyup.enter="$emit('confirmEdit', node.id)"
          @keyup.escape="$emit('cancelEdit')"
          ref="editInputRef"
        />
      </template>
      <template v-else>
        <span class="cat-name" :title="node.name">{{ node.name }}</span>
        <el-dropdown v-if="canWrite" trigger="click" @command="handleCatCommand" @click.stop class="cat-dropdown">
          <span class="cat-more" @click.stop>···</span>
          <template #dropdown>
            <el-dropdown-menu>
              <el-dropdown-item command="addApi">添加接口</el-dropdown-item>
              <el-dropdown-item command="addChild">添加子分类</el-dropdown-item>
              <el-dropdown-item command="edit">编辑</el-dropdown-item>
              <el-dropdown-item command="delete" divided>删除</el-dropdown-item>
            </el-dropdown-menu>
          </template>
        </el-dropdown>
      </template>
    </div>

    <!-- 展开子内容：扁平模式下由外层虚拟列表统一编排分类与接口，这里不再渲染任何子内容 -->
    <div v-if="node.expanded && !flat">
      <!-- 子分类（多级）：扁平模式下由外层虚拟列表渲染，这里不递归 -->
      <CategoryNode
        v-for="child in children"
        v-if="!flat"
        :key="child.id"
        :node="child"
        :depth="depth + 1"
        :selected-id="selectedId"
        :selected-request-id="selectedRequestId"
        :editing-id="editingId"
        :edit-name="editName"
        :adding-at="addingAt"
        :add-name="addName"
        @select="$emit('select', $event)"
        @start-add="$emit('startAdd', $event)"
        @start-add-api="$emit('startAddApi', $event)"
        @start-edit="$emit('startEdit', $event[0], $event[1])"
        @confirm-edit="$emit('confirmEdit', $event)"
        @cancel-edit="$emit('cancelEdit')"
        @delete="$emit('delete', $event[0], $event[1])"
        @toggle="$emit('toggle', $event)"
        @confirm-add="$emit('confirmAdd', $event)"
        @cancel-add="$emit('cancelAdd')"
        @update:add-name="$emit('update:addName', $event)"
        @update:edit-name="$emit('update:editName', $event)"
        @select-request="$emit('selectRequest', $event)"
        @delete-request="$emit('deleteRequest', $event)"
      />

      <!-- 该分类下的接口（Phase 7.6：条数超过阈值时只渲染可视窗口 + 上下占位） -->
      <div
        ref="apiListRef"
        class="api-list"
        :class="{ 'api-list--virtual': apiListVirtual }"
        :style="apiListPadStyle"
        @scroll.passive="onApiListScroll"
      >
        <div
          v-for="req in apiListItems"
          :key="req.id"
          class="api-node"
          :class="{ active: selectedRequestId === req.id }"
          :style="{ paddingLeft: (26 + (depth + 1) * 18) + 'px' }"
          @click="$emit('selectRequest', req)"
          @contextmenu.prevent="onApiContextMenu($event, req)"
        >
          <span class="expand-spacer" />
          <el-tag size="small" :type="methodTag(req.method)" effect="plain" class="api-method">
            {{ req.method }}
          </el-tag>
          <span class="api-name" :title="reqUrl(req)">{{ req.name }}</span>
          <el-dropdown v-if="canWrite" trigger="click" @command="(cmd: string) => handleApiCommand(cmd, req)" @click.stop class="api-dropdown">
            <span class="api-more" @click.stop>···</span>
            <template #dropdown>
              <el-dropdown-menu>
                <el-dropdown-item command="delete">删除</el-dropdown-item>
              </el-dropdown-menu>
            </template>
          </el-dropdown>
        </div>
      </div>
    </div>

    <!-- 新建子分类输入框 -->
    <div
      v-if="addingAt === node.id"
      class="cat-node cat-node--edit"
      :style="{ paddingLeft: (26 + (depth + 1) * 18) + 'px' }"
    >
      <el-icon><Folder /></el-icon>
      <el-input
        :model-value="addName"
        size="small"
        placeholder="子分类名"
        class="edit-input"
        @update:model-value="$emit('update:addName', $event)"
        @keyup.enter="$emit('confirmAdd', node.id)"
        @keyup.escape="$emit('cancelAdd')"
        ref="childAddInputRef"
      />
    </div>

    <!-- 右键菜单：放在节点根级（展开块之外），扁平模式下也能正常弹出 -->
    <Teleport to="body">
      <div
        v-if="ctxMenu.visible && canWrite"
        class="ctx-menu"
        :style="{ left: ctxMenu.x + 'px', top: ctxMenu.y + 'px' }"
        @click.stop
      >
        <template v-if="ctxMenu.req">
          <div class="ctx-menu-item" @click="ctxMenuEdit">编辑</div>
          <div class="ctx-menu-item ctx-menu-item--danger" @click="ctxMenuDelete">删除</div>
        </template>
        <template v-else-if="ctxMenu.node">
          <div class="ctx-menu-item" @click="ctxMenuAddApi">添加接口</div>
          <div class="ctx-menu-item" @click="ctxMenuAddChild">添加子分类</div>
          <div class="ctx-menu-item" @click="ctxMenuEdit">编辑</div>
          <div class="ctx-menu-item ctx-menu-item--danger" @click="ctxMenuDelete">删除</div>
        </template>
      </div>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, reactive, ref, watch } from 'vue'
import { useCategories } from '@/composables/useCategories'
import { useSavedRequests } from '@/composables/useSavedRequests'
import { useVirtualWindow } from '@/composables/useVirtualWindow'
import { canWrite } from '@/composables/useTeams'
import type { Category } from '@/types'
import type { SavedRequest } from '@/composables/useSavedRequests'

const ctxMenu = reactive<{ visible: boolean; x: number; y: number; req: SavedRequest | null; node: Category | null }>({
  visible: false, x: 0, y: 0, req: null, node: null,
})

function onApiContextMenu(e: MouseEvent, req: SavedRequest) {
  ctxMenu.visible = true
  ctxMenu.x = e.clientX
  ctxMenu.y = e.clientY
  ctxMenu.req = req
  ctxMenu.node = null
}

function onCatContextMenu(e: MouseEvent, node: Category) {
  ctxMenu.visible = true
  ctxMenu.x = e.clientX
  ctxMenu.y = e.clientY
  ctxMenu.req = null
  ctxMenu.node = node
  emit('select', node.id)
}

function ctxMenuAddApi() {
  if (ctxMenu.node) emit('startAddApi', ctxMenu.node.id)
  ctxMenu.visible = false
}
function ctxMenuAddChild() {
  if (ctxMenu.node) emit('startAdd', ctxMenu.node.id)
  ctxMenu.visible = false
}
function ctxMenuEdit() {
  if (ctxMenu.req) { emit('selectRequest', ctxMenu.req); ctxMenu.visible = false; return }
  if (ctxMenu.node) { emit('startEdit', ctxMenu.node.id, ctxMenu.node.name); ctxMenu.visible = false; return }
  ctxMenu.visible = false
}
function ctxMenuDelete() {
  if (ctxMenu.req) emit('deleteRequest', ctxMenu.req)
  else if (ctxMenu.node) emit('delete', ctxMenu.node.id, ctxMenu.node.name)
  ctxMenu.visible = false
}
function closeCtxMenu() { ctxMenu.visible = false }

onMounted(() => document.addEventListener('click', closeCtxMenu))
onUnmounted(() => document.removeEventListener('click', closeCtxMenu))

const draggingId = ref<string | null>(null)
const dragOverId = ref<string | null>(null)

const props = defineProps<{
  node: Category
  depth: number
  selectedId: string | null
  selectedRequestId?: string
  editingId: string | null
  editName: string
  addingAt: string | null
  addName: string
  /** 扁平模式：由外层虚拟列表统一编排，本组件不再递归渲染子分类（避免整棵树一次性全量上 DOM） */
  flat?: boolean
}>()

const emit = defineEmits<{
  (e: 'select', id: string): void
  (e: 'startAdd', parentId: string): void
  (e: 'startAddApi', categoryId: string): void
  (e: 'startEdit', id: string, name: string): void
  (e: 'confirmEdit', id: string): void
  (e: 'cancelEdit'): void
  (e: 'delete', id: string, name: string): void
  (e: 'toggle', id: string): void
  (e: 'confirmAdd', parentId: string): void
  (e: 'cancelAdd'): void
  (e: 'update:editName', v: string): void
  (e: 'update:addName', v: string): void
  (e: 'selectRequest', req: SavedRequest): void
  (e: 'deleteRequest', req: SavedRequest): void
}>()

function handleCatCommand(cmd: string) {
  switch (cmd) {
    case 'addApi': emit('startAddApi', props.node.id); break
    case 'addChild': emit('startAdd', props.node.id); break
    case 'edit': emit('startEdit', props.node.id, props.node.name); break
    case 'delete': emit('delete', props.node.id, props.node.name); break
  }
}

function handleApiCommand(cmd: string, req: SavedRequest) {
  if (cmd === 'delete') emit('deleteRequest', req)
}

const { getChildren } = useCategories()
const { getRequestsByCategory } = useSavedRequests()

function reqUrl(req: SavedRequest): string {
  return req.url || req.name || '未命名接口'
}

const children = computed(() => getChildren(props.node.id))
const requests = computed(() => getRequestsByCategory(props.node.id))

// ===== Phase 7.6：接口列表窗口化渲染 =====
// 背景：分类下的接口此前是全量 v-for，千级节点首渲染 > 1s。
// 策略：超过阈值才启用窗口化（并因此出现内层滚动条），普通分类保持原有展开式布局不变。
/** 接口行固定高度（px），必须与 `.api-list--virtual .api-node` 的 CSS 高度保持一致 */
const API_ROW_HEIGHT = 30
/** 启用虚拟滚动的条数阈值 */
const API_VIRTUAL_THRESHOLD = 60

const apiListRef = ref<HTMLElement | null>(null)
const apiListVirtual = computed(() => requests.value.length > API_VIRTUAL_THRESHOLD)
const {
  start: apiSliceStart,
  end: apiSliceEnd,
  topPad: apiTopPad,
  bottomPad: apiBottomPad,
  onScroll: onApiListScrollInner,
  syncViewport: syncApiListViewport,
} = useVirtualWindow({ total: () => requests.value.length, itemHeight: API_ROW_HEIGHT })

/** 虚拟模式下只渲染窗口内的行；否则全量渲染 */
const apiListItems = computed(() =>
  apiListVirtual.value ? requests.value.slice(apiSliceStart.value, apiSliceEnd.value) : requests.value,
)

/** 用上下 padding 占住被裁剪行的滚动空间（保持单一处行标记，避免模板重复） */
const apiListPadStyle = computed(() =>
  apiListVirtual.value
    ? { paddingTop: `${apiTopPad.value}px`, paddingBottom: `${apiBottomPad.value}px` }
    : undefined,
)

function onApiListScroll(e: Event) {
  if (apiListVirtual.value) onApiListScrollInner(e)
}

// 首次进入虚拟模式时容器刚刚创建，需测量视口高度，否则只会渲染 overscan 行
watch(
  apiListVirtual,
  async (on) => {
    if (!on) return
    await nextTick()
    syncApiListViewport(apiListRef.value)
  },
  { immediate: true },
)

const editInputRef = ref<any>(null)
const childAddInputRef = ref<any>(null)

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

watch(() => props.editingId, (val) => {
  if (val === props.node.id) nextTick(() => editInputRef.value?.focus?.())
})
watch(() => props.addingAt, (val) => {
  if (val === props.node.id) nextTick(() => childAddInputRef.value?.focus?.())
})
</script>

<style scoped>
.cat-node {
  display: flex;
  align-items: center;
  gap: 4px;
  padding-top: 5px;
  padding-bottom: 5px;
  padding-right: 6px;
  border-radius: 7px;
  cursor: pointer;
  font-size: var(--fs-md);
  color: var(--text-1, #0f172a);
  transition: all 0.12s cubic-bezier(0.16, 1, 0.3, 1);
  user-select: none;
}
.cat-node:hover { background: rgba(99,102,241,0.04); }
.cat-node.active {
  background: linear-gradient(135deg, rgba(99,102,241,.10), rgba(139,92,246,.05));
  color: #4f46e5;
  font-weight: 600;
  box-shadow: 0 1px 3px rgba(99,102,241,0.06);
}
.cat-node.drag-over {
  background: rgba(99,102,241,0.12);
  outline: 2px dashed var(--brand-1, #6366f1);
  outline-offset: -2px;
}
.cat-node.dragging { opacity: 0.4; }
.cat-node--edit { cursor: default; }
.cat-node--edit:hover { background: transparent; }
.expand-icon {
  width: 14px; height: 14px; font-size: var(--fs-2xs);
  color: var(--text-4, #94a3b8);
  transition: transform 0.2s cubic-bezier(0.16, 1, 0.3, 1);
  flex-shrink: 0;
}
.expand-icon.expanded { transform: rotate(90deg); }
.expand-spacer { width: 14px; height: 14px; flex-shrink: 0; }
.folder-icon { font-size: var(--fs-md); color: #f59e0b; flex-shrink: 0; }
.cat-name { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.cat-dropdown { display: none; flex-shrink: 0; }
.cat-node:hover .cat-dropdown { display: inline-flex; }
.cat-more {
  font-size: var(--fs-lg); color: var(--text-4, #94a3b8); cursor: pointer;
  padding: 0 2px; line-height: 1; letter-spacing: -1px; border-radius: 3px;
}
.cat-more:hover { color: var(--text-2, #334155); background: rgba(99,102,241,0.08); }
.edit-input { flex: 1; }

.api-node {
  display: flex; align-items: center; gap: 6px;
  padding-top: 5px; padding-bottom: 5px; padding-right: 6px;
  border-radius: 7px; cursor: pointer; font-size: var(--fs-sm);
  color: var(--text-2, #334155);
  transition: all 0.12s cubic-bezier(0.16, 1, 0.3, 1);
  user-select: none;
}
/* Phase 7.6：窗口化渲染要求行高固定，否则上下占位高度与真实滚动位置不匹配 */
.api-list--virtual { max-height: 320px; overflow-y: auto; }
.api-list--virtual .api-node {
  height: 30px;
  box-sizing: border-box;
  padding-top: 0;
  padding-bottom: 0;
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

.ctx-menu {
  position: fixed; z-index: 9999; background: var(--surface);
  border: 1px solid #e5e7eb; border-radius: 8px;
  box-shadow: 0 4px 16px rgba(0,0,0,.12); padding: 4px; min-width: 100px;
}
.ctx-menu-item {
  padding: 6px 14px; font-size: var(--fs-sm); color: var(--text-2);
  border-radius: 6px; cursor: pointer; transition: background .12s;
}
.ctx-menu-item:hover { background: rgba(99,102,241,0.08); color: #4f46e5; }
.ctx-menu-item--danger:hover { background: var(--tint-danger); color: #dc2626; }
</style>
