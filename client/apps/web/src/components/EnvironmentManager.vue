<template>
  <el-dialog
    v-model="visible"
    :title="currentProjectName ? `环境管理 · 项目：${currentProjectName}` : '环境管理'"
    width="680px"
    @closed="handleDialogClosed"
  >
    <div class="env-manager">
      <!-- 环境列表 -->
      <div class="env-list-panel">
        <div class="panel-header">
          <span class="panel-title">环境</span>
          <el-dropdown v-if="canWrite" trigger="click" @command="handleListHeaderCommand">
            <el-button size="small" link type="primary">
              <el-icon><Plus /></el-icon>
            </el-button>
            <template #dropdown>
              <el-dropdown-menu>
                <el-dropdown-item command="addGroup">新建分组</el-dropdown-item>
                <el-dropdown-item command="addEnv">新建环境</el-dropdown-item>
              </el-dropdown-menu>
            </template>
          </el-dropdown>
        </div>
        <el-scrollbar class="panel-scrollbar">
          <!-- 分组列表 -->
          <template v-for="group in envGroups" :key="group.id">
            <div class="env-group-header" @click="toggleGroupExpand(group)">
              <el-icon class="expand-icon" :class="{ expanded: group.expanded }">
                <ArrowRight />
              </el-icon>
              <template v-if="editingGroupId === group.id">
                <el-input
                  v-model="editingGroupName"
                  size="small"
                  class="group-name-input"
                  @keyup.enter="confirmEditGroup(group)"
                  @keyup.escape="cancelEditGroup"
                  @click.stop
                  ref="groupNameInputRef"
                />
                <el-button size="small" link type="primary" class="inline-save-btn" title="保存" @click="confirmEditGroup(group)">
                  <el-icon><Check /></el-icon>
                </el-button>
              </template>
              <template v-else>
                <span class="group-name">{{ group.name }}</span>
                <el-dropdown v-if="canWrite" trigger="click" @command="(cmd: string) => handleGroupCommand(cmd, group)" @click.stop class="group-dropdown">
                  <span class="group-more" @click.stop>···</span>
                  <template #dropdown>
                    <el-dropdown-menu>
                      <el-dropdown-item command="addEnv">新建环境</el-dropdown-item>
                      <el-dropdown-item command="rename">重命名</el-dropdown-item>
                      <el-dropdown-item command="delete" divided>删除分组</el-dropdown-item>
                    </el-dropdown-menu>
                  </template>
                </el-dropdown>
              </template>
            </div>
            <template v-if="group.expanded">
              <div
                v-for="env in getEnvsByGroup(group.id)"
                :key="env.id"
                class="env-item"
                :class="{ active: selectedEnvId === env.id, 'is-active': env.isActive }"
                @click="selectEnv(env.id)"
              >
                <el-icon v-if="env.isActive" class="env-active-icon"><Select /></el-icon>
                <span v-else class="env-active-spacer" />
                <template v-if="editingEnvId === env.id">
                  <el-input
                    v-model="editingEnvName"
                    size="small"
                    class="env-name-input"
                    @keyup.enter="confirmEditEnv(env)"
                    @keyup.escape="cancelEditEnv"
                    ref="envNameInputRef"
                  />
                  <el-button size="small" link type="primary" class="inline-save-btn" title="保存" @click="confirmEditEnv(env)">
                    <el-icon><Check /></el-icon>
                  </el-button>
                </template>
                <template v-else>
                  <span class="env-name">{{ env.name }}</span>
                  <el-dropdown v-if="canWrite" trigger="click" @command="(cmd: string) => handleEnvCommand(cmd, env)" @click.stop class="env-dropdown">
                    <span class="env-more" @click.stop>···</span>
                    <template #dropdown>
                      <el-dropdown-menu>
                        <el-dropdown-item command="rename">重命名</el-dropdown-item>
                        <template v-if="envGroups.filter(g => g.id !== env.groupId).length">
                          <el-dropdown-item divided disabled>移动到分组</el-dropdown-item>
                          <el-dropdown-item
                            v-for="g in envGroups.filter(g => g.id !== env.groupId)"
                            :key="g.id"
                            :command="`move:${g.id}`"
                          >&nbsp;&nbsp;{{ g.name }}</el-dropdown-item>
                          <el-dropdown-item
                            v-if="env.groupId"
                            command="move:null"
                          >&nbsp;&nbsp;未分组</el-dropdown-item>
                        </template>
                        <el-dropdown-item command="delete" divided>删除</el-dropdown-item>
                      </el-dropdown-menu>
                    </template>
                  </el-dropdown>
                </template>
              </div>
              <!-- 分组内新建环境输入 -->
              <div v-if="addingEnv && addingEnvGroupId === group.id" class="env-item env-item--edit">
                <span class="env-active-spacer" />
                <el-input
                  v-model="newEnvName"
                  size="small"
                  placeholder="环境名"
                  class="env-name-input"
                  @keyup.enter="confirmAddEnv"
                  @keyup.escape="cancelAddEnv"
                  ref="newEnvInputRef"
                />
                <el-button size="small" link type="primary" class="inline-save-btn" title="保存" @click="confirmAddEnv">
                  <el-icon><Check /></el-icon>
                </el-button>
              </div>
            </template>
          </template>

          <!-- 未分组 -->
          <template v-if="ungroupedEnvs.length || (addingEnv && addingEnvGroupId === null)">
            <div class="env-group-header" @click="ungroupedExpanded = !ungroupedExpanded">
              <el-icon class="expand-icon" :class="{ expanded: ungroupedExpanded }">
                <ArrowRight />
              </el-icon>
              <span class="group-name">未分组</span>
              <el-dropdown trigger="click" @command="(cmd: string) => handleUngroupedCommand(cmd)" @click.stop class="group-dropdown">
                <span class="group-more" @click.stop>···</span>
                <template #dropdown>
                  <el-dropdown-menu>
                    <el-dropdown-item command="addEnv">新建环境</el-dropdown-item>
                  </el-dropdown-menu>
                </template>
              </el-dropdown>
            </div>
            <template v-if="ungroupedExpanded">
              <div
                v-for="env in ungroupedEnvs"
                :key="env.id"
                class="env-item"
                :class="{ active: selectedEnvId === env.id, 'is-active': env.isActive }"
                @click="selectEnv(env.id)"
              >
                <el-icon v-if="env.isActive" class="env-active-icon"><Select /></el-icon>
                <span v-else class="env-active-spacer" />
                <template v-if="editingEnvId === env.id">
                  <el-input
                    v-model="editingEnvName"
                    size="small"
                    class="env-name-input"
                    @keyup.enter="confirmEditEnv(env)"
                    @keyup.escape="cancelEditEnv"
                    ref="envNameInputRef"
                  />
                  <el-button size="small" link type="primary" class="inline-save-btn" title="保存" @click="confirmEditEnv(env)">
                    <el-icon><Check /></el-icon>
                  </el-button>
                </template>
                <template v-else>
                  <span class="env-name">{{ env.name }}</span>
                  <el-dropdown trigger="click" @command="(cmd: string) => handleEnvCommand(cmd, env)" @click.stop class="env-dropdown">
                    <span class="env-more" @click.stop>···</span>
                    <template #dropdown>
                      <el-dropdown-menu>
                        <el-dropdown-item command="rename">重命名</el-dropdown-item>
                        <template v-if="envGroups.length">
                          <el-dropdown-item divided disabled>移动到分组</el-dropdown-item>
                          <el-dropdown-item
                            v-for="g in envGroups"
                            :key="g.id"
                            :command="`move:${g.id}`"
                          >&nbsp;&nbsp;{{ g.name }}</el-dropdown-item>
                        </template>
                        <el-dropdown-item command="delete" divided>删除</el-dropdown-item>
                      </el-dropdown-menu>
                    </template>
                  </el-dropdown>
                </template>
              </div>
              <!-- 未分组内新建环境输入 -->
              <div v-if="addingEnv && addingEnvGroupId === null" class="env-item env-item--edit">
                <span class="env-active-spacer" />
                <el-input
                  v-model="newEnvName"
                  size="small"
                  placeholder="环境名"
                  class="env-name-input"
                  @keyup.enter="confirmAddEnv"
                  @keyup.escape="cancelAddEnv"
                  ref="newEnvInputRef"
                />
                <el-button size="small" link type="primary" class="inline-save-btn" title="保存" @click="confirmAddEnv">
                  <el-icon><Check /></el-icon>
                </el-button>
              </div>
            </template>
          </template>

          <!-- 新建分组输入 -->
          <div v-if="addingGroup" class="env-group-header env-group-header--edit">
            <el-icon><Folder /></el-icon>
            <el-input
              v-model="newGroupName"
              size="small"
              placeholder="分组名"
              class="group-name-input"
              @keyup.enter="confirmAddGroup"
              @keyup.escape="cancelAddGroup"
              ref="newGroupInputRef"
            />
            <el-button size="small" link type="primary" class="inline-save-btn" title="保存" @click="confirmAddGroup">
              <el-icon><Check /></el-icon>
            </el-button>
          </div>

          <!-- 空状态 -->
          <EmptyState
            v-if="!envGroups.length && !ungroupedEnvs.length && !addingGroup"
            compact
            title="暂无环境"
            description="点击右上角 + 添加"
          />
        </el-scrollbar>
      </div>

      <!-- 变量列表 -->
      <div class="env-vars-panel">
        <div class="panel-header">
          <span class="panel-title">
            变量<span v-if="selectedEnv"> — {{ selectedEnv.name }}</span>
          </span>
          <div class="panel-header-right">
            <el-button
              v-if="canWrite && selectedEnv && selectedEnv.id !== activeEnv?.id"
              size="small"
              type="primary"
              plain
              @click="handleActivate(selectedEnv.id)"
            >
              激活此环境
            </el-button>
            <el-button v-if="canWrite" size="small" link type="primary" @click="addVar">
              <el-icon><Plus /></el-icon>
            </el-button>
          </div>
        </div>
        <el-scrollbar v-if="selectedEnvId" class="panel-scrollbar">
          <EmptyState v-if="!variables.length && !addingVar" compact title="暂无变量" description="点击 + 添加变量" />
          <div
            v-for="v in variables"
            :key="v.id"
            class="var-row"
          >
            <el-checkbox
              :model-value="v.enabled"
              :disabled="!canWrite"
              @update:model-value="(val: boolean) => toggleVarEnabled(v, val)"
              size="small"
            />
            <el-input
              :model-value="v.key"
              :disabled="!canWrite"
              size="small"
              placeholder="变量名"
              class="var-key"
              @update:model-value="(val: string) => updateVarKey(v, val)"
            />
            <el-input
              :model-value="v.value"
              :disabled="!canWrite"
              size="small"
              placeholder="变量值"
              class="var-val"
              @update:model-value="(val: string) => updateVarValue(v, val)"
            />
            <el-button v-if="canWrite" size="small" link type="primary" class="inline-save-btn" title="保存" @click="saveVar(v)">
              <el-icon><Check /></el-icon>
            </el-button>
            <el-button v-if="canWrite" link type="danger" size="small" @click="handleDeleteVar(v)">
              <el-icon><Delete /></el-icon>
            </el-button>
          </div>
          <!-- 新建变量行 -->
          <div v-if="addingVar" class="var-row">
            <el-checkbox :model-value="true" size="small" disabled />
            <el-input
              v-model="newVarKey"
              size="small"
              placeholder="变量名"
              class="var-key"
              @keyup.enter="confirmAddVar"
              ref="newVarKeyInputRef"
            />
            <el-input
              v-model="newVarValue"
              size="small"
              placeholder="变量值"
              class="var-val"
              @keyup.enter="confirmAddVar"
            />
            <el-button size="small" link type="primary" class="inline-save-btn" title="保存" @click="confirmAddVar">
              <el-icon><Check /></el-icon>
            </el-button>
            <el-button link type="danger" size="small" @click="cancelAddVar">
              <el-icon><Close /></el-icon>
            </el-button>
          </div>
        </el-scrollbar>
        <EmptyState v-else compact title="请选择一个环境" />
      </div>
    </div>

    <template #footer>
      <el-button @click="visible = false">关闭</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import EmptyState from '@/components/EmptyState.vue'
import { computed, nextTick, ref } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { useEnvironments } from '@/composables/useEnvironments'
import type { Environment, EnvironmentGroup } from '@/composables/useEnvironments'
import type { EnvironmentVariable } from '@/types'
import { canWrite } from '@/composables/useTeams'
import { useProjects } from '@/composables/useProjects'

function handleDialogClosed() {
  // 关闭时自动提交未完成的编辑，避免数据丢失
  if (editingEnvId.value) {
    const env = environments.value.find(e => e.id === editingEnvId.value)
    if (env) {
      const name = editingEnvName.value.trim()
      if (name && name !== env.name) {
        env.name = name
        updateEnv(env)
      }
    }
    editingEnvId.value = null
  }
  if (editingGroupId.value) {
    const group = envGroups.value.find(g => g.id === editingGroupId.value)
    if (group) {
      const name = editingGroupName.value.trim()
      if (name && name !== group.name) {
        group.name = name
        updateGroup(group)
      }
    }
    editingGroupId.value = null
  }
  addingEnv.value = false
  addingGroup.value = false
  emit('closed')
}

const emit = defineEmits<{
  (e: 'closed'): void
}>()

const {
  environments,
  envGroups,
  activeEnv,
  load,
  createEnv,
  updateEnv,
  deleteEnv,
  activateEnv,
  getVariables,
  saveVariable,
  updateVariable,
  deleteVariable,
  createGroup,
  updateGroup,
  deleteGroup,
  toggleGroupExpand,
  moveEnvToGroup,
} = useEnvironments()

const { projects, selectedProjectId } = useProjects()
const currentProjectName = computed(() =>
  selectedProjectId.value ? (projects.find(p => p.id === selectedProjectId.value)?.name ?? '') : ''
)

const visible = ref(false)
const selectedEnvId = ref<string | null>(null)
const variables = ref<EnvironmentVariable[]>([])
const addingEnv = ref(false)
const addingEnvGroupId = ref<string | null>(null)
const newEnvName = ref('')
const newEnvInputRef = ref<any>(null)
const editingEnvId = ref<string | null>(null)
const editingEnvName = ref('')
const envNameInputRef = ref<any>(null)
const addingVar = ref(false)
const newVarKey = ref('')
const newVarValue = ref('')
const newVarKeyInputRef = ref<any>(null)

// 分组状态
const addingGroup = ref(false)
const newGroupName = ref('')
const newGroupInputRef = ref<any>(null)
const editingGroupId = ref<string | null>(null)
const editingGroupName = ref('')
const groupNameInputRef = ref<any>(null)
const ungroupedExpanded = ref(true)

const selectedEnv = computed(() => environments.value.find(e => e.id === selectedEnvId.value) ?? null)
const ungroupedEnvs = computed(() => environments.value.filter(e => !e.groupId))

function getEnvsByGroup(groupId: string) {
  return environments.value.filter(e => e.groupId === groupId)
}

function open() {
  visible.value = true
  load()
  if (!selectedEnvId.value && environments.value.length) {
    selectEnv(activeEnv.value?.id || environments.value[0].id)
  }
}

async function selectEnv(envId: string) {
  selectedEnvId.value = envId
  variables.value = await getVariables(envId)
}

// ====== 列表头部下拉 ======
function handleListHeaderCommand(cmd: string) {
  if (cmd === 'addGroup') {
    startAddGroup()
  } else if (cmd === 'addEnv') {
    startAddEnv(null)
  }
}

// ====== 分组操作 ======
function startAddGroup() {
  addingGroup.value = true
  newGroupName.value = ''
  nextTick(() => newGroupInputRef.value?.focus?.())
}

async function confirmAddGroup() {
  const name = newGroupName.value.trim()
  if (!name) { addingGroup.value = false; return }
  await createGroup(name)
  addingGroup.value = false
  newGroupName.value = ''
  ElMessage.success(`分组「${name}」已创建`)
}

function cancelAddGroup() {
  addingGroup.value = false
  newGroupName.value = ''
}

function startEditGroup(group: EnvironmentGroup) {
  nextTick(() => {
    editingGroupId.value = group.id
    editingGroupName.value = group.name
    nextTick(() => groupNameInputRef.value?.focus?.())
  })
}

async function confirmEditGroup(group: EnvironmentGroup) {
  if (editingGroupId.value !== group.id) return
  const name = editingGroupName.value.trim()
  if (!name || name === group.name) { cancelEditGroup(); return }
  group.name = name
  await updateGroup(group)
  cancelEditGroup()
}

function cancelEditGroup() {
  editingGroupId.value = null
  editingGroupName.value = ''
}

async function handleDeleteGroup(group: EnvironmentGroup) {
  try {
    await ElMessageBox.confirm(
      `确定要删除分组「${group.name}」吗？分组内的环境将变为未分组。`,
      '删除分组',
      { confirmButtonText: '删除', cancelButtonText: '取消', type: 'warning' },
    )
    await deleteGroup(group.id)
    ElMessage.success('分组已删除')
  } catch { /* cancelled */ }
}

function handleGroupCommand(cmd: string, group: EnvironmentGroup) {
  switch (cmd) {
    case 'addEnv': startAddEnv(group.id); break
    case 'rename': startEditGroup(group); break
    case 'delete': handleDeleteGroup(group); break
  }
}

function handleUngroupedCommand(cmd: string) {
  if (cmd === 'addEnv') startAddEnv(null)
}

// ====== 环境操作 ======
function startAddEnv(groupId: string | null) {
  addingEnv.value = true
  addingEnvGroupId.value = groupId
  newEnvName.value = ''
  // 确保目标分组区域展开
  if (groupId === null) {
    ungroupedExpanded.value = true
  } else {
    const group = envGroups.value.find(g => g.id === groupId)
    if (group) group.expanded = true
  }
  nextTick(() => newEnvInputRef.value?.focus?.())
}

async function confirmAddEnv() {
  const name = newEnvName.value.trim()
  if (!name) { addingEnv.value = false; return }
  const env = await createEnv(name, addingEnvGroupId.value)
  addingEnv.value = false
  addingEnvGroupId.value = null
  newEnvName.value = ''
  selectEnv(env.id)
  ElMessage.success(`环境「${name}」已创建`)
}

function cancelAddEnv() {
  addingEnv.value = false
  addingEnvGroupId.value = null
  newEnvName.value = ''
}

function startEditEnv(env: Environment) {
  nextTick(() => {
    editingEnvId.value = env.id
    editingEnvName.value = env.name
    nextTick(() => envNameInputRef.value?.focus?.())
  })
}

async function confirmEditEnv(env: Environment) {
  if (editingEnvId.value !== env.id) return
  const name = editingEnvName.value.trim()
  if (!name || name === env.name) { cancelEditEnv(); return }
  env.name = name
  await updateEnv(env)
  cancelEditEnv()
}

function cancelEditEnv() {
  editingEnvId.value = null
  editingEnvName.value = ''
}

async function handleDeleteEnv(env: Environment) {
  try {
    await ElMessageBox.confirm(
      `确定要删除环境「${env.name}」及其所有变量吗？`,
      '删除环境',
      { confirmButtonText: '删除', cancelButtonText: '取消', type: 'warning' },
    )
    await deleteEnv(env.id)
    if (selectedEnvId.value === env.id) {
      selectedEnvId.value = null
      variables.value = []
    }
    ElMessage.success('已删除')
  } catch { /* cancelled */ }
}

function handleEnvCommand(cmd: string, env: Environment) {
  if (cmd.startsWith('move:')) {
    const targetGroupId = cmd.slice(5)
    moveEnvToGroup(env.id, targetGroupId === 'null' ? null : targetGroupId)
    return
  }
  switch (cmd) {
    case 'rename': startEditEnv(env); break
    case 'delete': handleDeleteEnv(env); break
  }
}

async function handleActivate(id: string) {
  await activateEnv(id)
  ElMessage.success('环境已激活')
}

// ====== 变量操作 ======
function addVar() {
  if (!selectedEnvId.value) return
  addingVar.value = true
  newVarKey.value = ''
  newVarValue.value = ''
  nextTick(() => newVarKeyInputRef.value?.focus?.())
}

async function confirmAddVar() {
  const key = newVarKey.value.trim()
  if (!key) { addingVar.value = false; return }
  await saveVariable(selectedEnvId.value!, key, newVarValue.value)
  variables.value = await getVariables(selectedEnvId.value!)
  addingVar.value = false
  newVarKey.value = ''
  newVarValue.value = ''
}

function cancelAddVar() {
  addingVar.value = false
  newVarKey.value = ''
  newVarValue.value = ''
}

// 显式保存某条变量的当前改动（与即时保存语义一致，供保存图标调用）
async function saveVar(v: EnvironmentVariable) {
  if (!selectedEnvId.value) return
  await updateVariable(v)
  ElMessage.success('变量已保存')
}

async function toggleVarEnabled(v: EnvironmentVariable, val: boolean) {
  v.enabled = val
  await updateVariable(v)
}

async function updateVarKey(v: EnvironmentVariable, val: string) {
  v.key = val
  await updateVariable(v)
}

async function updateVarValue(v: EnvironmentVariable, val: string) {
  v.value = val
  await updateVariable(v)
}

async function handleDeleteVar(v: EnvironmentVariable) {
  await deleteVariable(v.id, v.environmentId)
  variables.value = variables.value.filter(x => x.id !== v.id)
}

defineExpose({ open })
</script>

<style scoped>
.env-manager {
  display: grid;
  grid-template-columns: 210px 1fr;
  grid-template-rows: 1fr;
  gap: 16px;
  min-height: 300px;
  max-height: 50vh;
  overflow: hidden;
}
.env-list-panel {
  border-right: 1px solid var(--divider, #ebeef3);
  padding-right: 12px;
  display: flex;
  flex-direction: column;
  min-height: 0;
  overflow: hidden;
}
.env-vars-panel {
  padding-left: 4px;
  display: flex;
  flex-direction: column;
  min-height: 0;
  overflow: hidden;
}
.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 8px;
  padding-bottom: 8px;
  border-bottom: 1px solid var(--divider, #ebeef3);
  flex-shrink: 0;
}
.panel-header-right {
  display: flex;
  align-items: center;
  gap: 4px;
}
.panel-title {
  font-weight: 700;
  font-size: var(--fs-md);
  color: var(--text-1, #0f172a);
  letter-spacing: -0.02em;
}
.env-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 8px 6px 22px;
  border-radius: 7px;
  cursor: pointer;
  font-size: var(--fs-md);
  color: var(--text-1, #0f172a);
  transition: all 0.12s cubic-bezier(0.16, 1, 0.3, 1);
  margin-bottom: 2px;
}
.env-item:hover { background: rgba(99,102,241,0.04); }
.env-item.active { background: linear-gradient(135deg, rgba(99,102,241,.10), rgba(139,92,246,.05)); color: #4f46e5; font-weight: 600; }
.env-item--edit { cursor: default; }
.env-item--edit:hover { background: transparent; }
.env-active-icon { color: #10b981; font-size: var(--fs-md); flex-shrink: 0; }
.env-active-spacer { width: 13px; height: 13px; flex-shrink: 0; }
.env-name {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.env-name-input { flex: 1; }
.inline-save-btn { flex-shrink: 0; margin-left: 2px; }
.env-dropdown {
  display: none;
  flex-shrink: 0;
}
.env-item:hover .env-dropdown { display: inline-flex; }
.env-more {
  font-size: var(--fs-lg);
  color: var(--text-4, #94a3b8);
  cursor: pointer;
  padding: 0 2px;
  line-height: 1;
  letter-spacing: -1px;
  border-radius: 3px;
}
.env-more:hover { color: var(--text-2, #334155); background: rgba(99,102,241,0.08); }
.env-item.is-active .env-name { font-weight: 600; }

/* 分组头部 */
.env-group-header {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 6px 8px;
  border-radius: 7px;
  cursor: pointer;
  font-size: var(--fs-sm);
  font-weight: 600;
  color: var(--text-3, #64748b);
  text-transform: uppercase;
  letter-spacing: 0.03em;
  margin-top: 4px;
  user-select: none;
  transition: background 0.12s;
}
.env-group-header:hover { background: rgba(99,102,241,0.04); }
.env-group-header--edit { cursor: default; }
.env-group-header--edit:hover { background: transparent; }
.expand-icon {
  width: 12px;
  height: 12px;
  font-size: var(--fs-2xs);
  color: var(--text-4, #94a3b8);
  transition: transform 0.2s cubic-bezier(0.16, 1, 0.3, 1);
  flex-shrink: 0;
}
.expand-icon.expanded { transform: rotate(90deg); }
.group-name {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.group-name-input { flex: 1; }
.group-dropdown {
  display: none;
  flex-shrink: 0;
}
.env-group-header:hover .group-dropdown { display: inline-flex; }
.group-more {
  font-size: var(--fs-lg);
  color: var(--text-4, #94a3b8);
  cursor: pointer;
  padding: 0 2px;
  line-height: 1;
  letter-spacing: -1px;
  border-radius: 3px;
}
.group-more:hover { color: var(--text-2, #334155); background: rgba(99,102,241,0.08); }
.env-empty {
  color: var(--text-4, #94a3b8);
  font-size: var(--fs-md);
  text-align: center;
  padding: 40px 0;
}
.var-row {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 6px;
  padding: 3px 4px;
  border-radius: 6px;
  transition: background 0.12s cubic-bezier(0.16, 1, 0.3, 1);
}
.var-row:hover { background: rgba(99,102,241,0.03); }
.var-key { flex: 1; max-width: 130px; }
.var-key :deep(input) {
  font-family: 'JetBrains Mono', ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: var(--fs-sm);
  letter-spacing: -0.01em;
}
.var-val { flex: 2; }
.var-val :deep(input) {
  font-size: var(--fs-sm);
}
.vars-empty {
  color: var(--text-4, #94a3b8);
  font-size: var(--fs-md);
  text-align: center;
  padding: 40px 0;
}
.panel-scrollbar {
  flex: 1;
  min-height: 0;
  overflow: hidden;
}
.panel-scrollbar :deep(.el-scrollbar__wrap) {
  overflow: auto;
}
</style>
