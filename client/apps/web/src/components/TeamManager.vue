<template>
  <div class="team-manager">
    <div v-if="!isLoggedIn" class="wm-login-hint">
      <el-icon><WarningFilled /></el-icon>
      请先在全局设置中配置服务器并登录
    </div>

    <div v-else class="wm-layout">
      <!-- 左侧：团队列表 -->
      <div class="wm-left">
        <div class="wm-section-title">
          我的团队
          <el-button size="small" link type="primary" @click="showCreate = true">
            <el-icon><Plus /></el-icon>
          </el-button>
        </div>

        <!-- 创建团队表单 -->
        <div v-if="showCreate" class="wm-inline-form">
          <el-input v-model="newWsName" size="small" placeholder="团队名称" />
          <el-input v-model="newWsDesc" size="small" placeholder="描述（可选）" />
          <div class="wm-inline-actions">
            <el-button size="small" @click="showCreate = false">取消</el-button>
            <el-button size="small" type="primary" @click="doCreate" :loading="creating">
              创建
            </el-button>
          </div>
        </div>

        <div class="wm-list">
          <div
              v-for="ws in teams"
              :key="ws.id"
              class="wm-ws-item"
              :class="{ active: activeWsId === ws.id }"
              @click="selectWs(ws.id)"
          >
            <div class="wm-ws-info">
              <span class="wm-ws-name">{{ ws.name }}</span>
              <el-tag
                  v-if="ws.ownerId === (currentUser?.id || '')"
                  size="small"
                  type="danger"
                  effect="plain"
              >owner</el-tag>
              <span v-if="ws.id === 'default'" class="wm-ws-badge">默认</span>
            </div>
            <div v-if="activeWsId === ws.id && ws.id !== 'default'" class="wm-ws-actions">
              <el-button
                  size="small"
                  link
                  @click.stop="startEdit(ws)"
                  :disabled="!canEditTeam(ws)"
                  :title="canEditTeam(ws) ? '编辑' : '仅所有者或管理员可编辑'"
              >
                <el-icon><EditPen /></el-icon>
              </el-button>
              <el-button
                  size="small"
                  link
                  type="danger"
                  @click.stop="confirmDelete(ws)"
                  :disabled="ws.ownerId !== (currentUser?.id || '')"
                  :title="ws.ownerId !== (currentUser?.id || '') ? '仅所有者可删除' : '删除团队'"
              >
                <el-icon><Delete /></el-icon>
              </el-button>
            </div>
          </div>
        </div>

        <!-- 编辑团队内联表单 -->
        <div v-if="editingWs" class="wm-inline-form">
          <el-input v-model="editWsName" size="small" placeholder="团队名称" />
          <el-input v-model="editWsDesc" size="small" placeholder="描述" />
          <div class="wm-inline-actions">
            <el-button size="small" @click="editingWs = null">取消</el-button>
            <el-button size="small" type="primary" @click="doUpdate" :loading="updating">保存</el-button>
          </div>
        </div>
      </div>

      <!-- 右侧：成员管理 -->
      <div class="wm-right">
        <div class="wm-section-title">成员管理</div>

        <div v-if="activeWsId" class="wm-members-section">
          <!-- 邀请（仅 owner/admin 可见） -->
          <div v-if="isAdminOrAbove" class="wm-invite-row">
            <el-select
                v-model="inviteRole"
                size="small"
                style="width: 95px"
                class="wm-role-select"
            >
              <el-option
                  v-for="r in MEMBER_ROLES"
                  :key="r.value"
                  :label="r.label"
                  :value="r.value"
              />
            </el-select>
            <el-input
                v-model="inviteUsername"
                size="small"
                placeholder="输入用户名"
                clearable
                @keyup.enter="doInvite"
            />
            <el-button size="small" type="primary" @click="doInvite" :loading="inviting">
              邀请
            </el-button>
          </div>

          <!-- 移交（仅 owner 可见） -->
          <div class="wm-transfer-row" v-if="isOwner && activeWsId !== 'default'">
            <el-popover placement="top" :width="280" trigger="click">
              <template #reference>
                <el-button size="small" link type="warning">
                  <el-icon><Switch /></el-icon>
                  移交所有权
                </el-button>
              </template>
              <div>
                <div class="wm-transfer-title">移交给其他成员</div>
                <el-input
                    v-model="transferUsername"
                    size="small"
                    placeholder="输入新 owner 用户名"
                    style="margin-bottom: 8px;"
                />
                <el-button size="small" type="primary" @click="doTransfer" :loading="transferring">
                  确认移交
                </el-button>
              </div>
            </el-popover>
          </div>

          <!-- 成员列表 -->
          <div class="wm-members-list">
            <EmptyState v-if="members.length === 0" compact title="暂无成员" />
            <div v-for="m in members" :key="m.userId" class="wm-member-item">
              <div class="wm-member-avatar">
                <el-icon><UserFilled /></el-icon>
              </div>
              <div class="wm-member-info">
                <span class="wm-member-name">{{ m.username }}</span>
                <el-tag
                    size="small"
                    :type="ROLE_TAG_TYPE[m.role]"
                    effect="plain"
                >{{ ROLE_LABEL_MAP[m.role] }}</el-tag>
              </div>
              <!-- 管理操作（仅 owner/admin 可见，且不能操作自己或 owner） -->
              <div
                  v-if="isAdminOrAbove && m.userId !== currentUser?.id && m.role !== 'owner'"
                  class="wm-member-actions"
              >
                <el-select
                    :model-value="m.role"
                    size="small"
                    style="width: 88px"
                    @change="(role: string) => handleChangeRole(m.userId, role as MemberRole)"
                >
                  <el-option
                      v-for="r in availableRoles"
                      :key="r.value"
                      :label="r.label"
                      :value="r.value"
                  />
                </el-select>
                <el-button
                    size="small"
                    link
                    type="danger"
                    :disabled="m.role === 'admin' && !isOwner"
                    :title="m.role === 'admin' && !isOwner ? '仅所有者可移除管理员' : '移除成员'"
                    @click="handleRemoveMember(m)"
                >
                  <el-icon><Delete /></el-icon>
                </el-button>
              </div>
            </div>
          </div>
        </div>

        <EmptyState v-else compact title="请选择一个团队" />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import EmptyState from '@/components/EmptyState.vue'
import { ref, computed, onMounted, watch } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { WarningFilled, Plus, EditPen, Delete, Switch, UserFilled } from '@element-plus/icons-vue'
import type { MemberRole } from '@/composables/useTeams'
import {
  teams,
  teamMembers as members,
  MEMBER_ROLES,
  ROLE_LABEL_MAP,
  ROLE_TAG_TYPE,
  loadTeams,
  createTeam,
  updateTeam,
  deleteTeam,
  inviteMember,
  changeMemberRole,
  removeMember,
  transferTeam,
  loadTeamMembers,
} from '@/composables/useTeams'
import { isLoggedIn, currentUser } from '@/composables/useSync'

// 新建团队
const showCreate = ref(false)
const newWsName = ref('')
const newWsDesc = ref('')
const creating = ref(false)

// 编辑团队
const editingWs = ref<string | null>(null)
const editWsName = ref('')
const editWsDesc = ref('')
const updating = ref(false)

// 当前选中的团队（组件内独立跟踪）
const activeWsId = ref('')

// 邀请（角色默认读写）
const inviteUsername = ref('')
const inviteRole = ref<MemberRole>('readwrite')
const inviting = ref(false)

// 移交
const transferUsername = ref('')
const transferring = ref(false)

// 当前用户在当前团队的角色
const currentUserRole = computed<MemberRole | null>(() => {
  const m = members.value.find(m => m.userId === currentUser.value?.id)
  return m?.role ?? null
})

const isOwner = computed(() => currentUserRole.value === 'owner')
const isAdminOrAbove = computed(() => currentUserRole.value === 'owner' || currentUserRole.value === 'admin')

// owner 可选所有角色；admin 只能选 readwrite/readonly
const availableRoles = computed(() => {
  if (isOwner.value) {
    return MEMBER_ROLES
  }
  return MEMBER_ROLES.filter(r => r.value !== 'admin')
})

/** 父组件（如切换 tab / 进入页面）调用，刷新团队与成员数据 */
async function refresh() {
  if (!isLoggedIn.value) return
  await loadTeams()
  activeWsId.value = teams.value[0]?.id ?? ''
  if (activeWsId.value) {
    await loadTeamMembers(activeWsId.value)
  }
}

onMounted(refresh)
// 登录态变化时（如登录/登出）自动刷新
watch(isLoggedIn, (v) => {
  if (v) refresh()
  else {
    activeWsId.value = 'default'
    members.value = []
  }
})

defineExpose({ refresh })

function selectWs(id: string) {
  activeWsId.value = id
}

function canEditTeam(ws: { ownerId?: string }) {
  if (ws.ownerId === currentUser.value?.id) return true
  return isAdminOrAbove.value
}

async function doCreate() {
  if (!newWsName.value.trim()) {
    ElMessage.warning('请输入团队名称')
    return
  }
  creating.value = true
  try {
    await createTeam(newWsName.value.trim(), newWsDesc.value.trim())
    ElMessage.success('团队创建成功')
    showCreate.value = false
    newWsName.value = ''
    newWsDesc.value = ''
  } catch (e: any) {
    ElMessage.error(typeof e === 'string' ? e : '创建失败')
  } finally {
    creating.value = false
  }
}

function startEdit(ws: { id: string; name: string; description?: string }) {
  editingWs.value = ws.id
  editWsName.value = ws.name
  editWsDesc.value = ws.description || ''
}

async function doUpdate() {
  if (!editingWs.value || !editWsName.value.trim()) return
  updating.value = true
  try {
    await updateTeam(editingWs.value, editWsName.value.trim(), editWsDesc.value.trim())
    ElMessage.success('团队已更新')
    editingWs.value = null
  } catch (e: any) {
    ElMessage.error(typeof e === 'string' ? e : '更新失败')
  } finally {
    updating.value = false
  }
}

async function confirmDelete(ws: { id: string; name: string }) {
  try {
    await ElMessageBox.confirm(
        `确定删除团队「${ws.name}」吗？此操作不可恢复。`,
        '删除确认',
        {
          type: 'warning',
          confirmButtonText: '删除',
          cancelButtonText: '取消',
        }
    )
    await deleteTeam(ws.id)
    ElMessage.success('团队已删除')
    if (activeWsId.value === ws.id) {
      activeWsId.value = 'default'
    }
  } catch {
    // 取消
  }
}

async function doInvite() {
  if (!inviteUsername.value.trim()) {
    ElMessage.warning('请输入用户名')
    return
  }
  inviting.value = true
  try {
    const msg = await inviteMember(activeWsId.value, inviteUsername.value.trim(), inviteRole.value)
    ElMessage.success(msg)
    inviteUsername.value = ''
  } catch (e: any) {
    ElMessage.error(typeof e === 'string' ? e : '邀请失败')
  } finally {
    inviting.value = false
  }
}

async function handleChangeRole(userId: string, newRole: MemberRole) {
  try {
    const msg = await changeMemberRole(activeWsId.value, userId, newRole)
    ElMessage.success(msg)
  } catch (e: any) {
    ElMessage.error(typeof e === 'string' ? e : '角色修改失败')
  }
}

async function handleRemoveMember(m: { userId: string; username: string }) {
  try {
    await ElMessageBox.confirm(
        `确定将「${m.username}」移出团队吗？`,
        '移除成员',
        { type: 'warning', confirmButtonText: '移除', cancelButtonText: '取消' }
    )
    const msg = await removeMember(activeWsId.value, m.userId)
    ElMessage.success(msg)
  } catch {
    // 取消或错误
  }
}

async function doTransfer() {
  if (!transferUsername.value.trim()) {
    ElMessage.warning('请输入新 owner 用户名')
    return
  }
  try {
    await ElMessageBox.confirm(
        `确定将团队所有权移交给「${transferUsername.value}」吗？移交后您将变为管理员。`,
        '确认移交',
        { type: 'warning', confirmButtonText: '确认移交', cancelButtonText: '取消' }
    )
  } catch {
    return
  }
  transferring.value = true
  try {
    const msg = await transferTeam(activeWsId.value, transferUsername.value.trim())
    ElMessage.success(msg)
    transferUsername.value = ''
  } catch (e: any) {
    ElMessage.error(typeof e === 'string' ? e : '移交失败')
  } finally {
    transferring.value = false
  }
}
</script>

<style scoped>
.wm-login-hint {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 24px;
  color: #e6a23c;
  font-size: var(--fs-lg);
  background: var(--tint-warning);
  border-radius: 8px;
  justify-content: center;
}
.wm-layout {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 20px;
  min-height: 340px;
}
.wm-left, .wm-right {
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 14px;
  background: var(--surface-2);
}
.wm-section-title {
  font-weight: 700;
  font-size: var(--fs-lg);
  color: var(--text-1);
  margin-bottom: 10px;
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.wm-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
  max-height: 240px;
  overflow-y: auto;
}
.wm-ws-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 10px;
  border-radius: 6px;
  cursor: pointer;
  transition: background 0.15s;
}
.wm-ws-item:hover { background: var(--surface-2); }
.wm-ws-item.active { background: var(--brand-soft); border: 1px solid var(--brand-1); }
.wm-ws-info { display: flex; align-items: center; gap: 6px; }
.wm-ws-name { font-size: var(--fs-md); color: var(--text-2); font-weight: 500; }
.wm-ws-badge { font-size: var(--fs-xs); color: #94a3b8; }
.wm-ws-actions { display: flex; gap: 2px; }
.wm-inline-form {
  margin-top: 10px;
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 10px;
  background: var(--surface-2);
  border-radius: 6px;
  border: 1px dashed #cbd5e1;
}
.wm-inline-actions {
  display: flex;
  justify-content: flex-end;
  gap: 6px;
}
.wm-invite-row {
  display: flex;
  gap: 6px;
  margin-bottom: 12px;
}
.wm-transfer-row {
  margin-bottom: 12px;
}
.wm-transfer-title {
  font-size: var(--fs-md);
  font-weight: 600;
  margin-bottom: 8px;
  color: var(--text-1);
}
.wm-members-list {
  max-height: 200px;
  overflow-y: auto;
}
.wm-member-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 6px;
  border-bottom: 1px solid var(--border-soft);
}
.wm-member-avatar {
  width: 30px;
  height: 30px;
  border-radius: 50%;
  background: var(--border);
  display: flex;
  align-items: center;
  justify-content: center;
  color: #64748b;
  flex-shrink: 0;
}
.wm-member-info {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 1;
  min-width: 0;
}
.wm-member-name { font-size: var(--fs-md); color: var(--text-1); }
.wm-member-actions {
  display: flex;
  align-items: center;
  gap: 4px;
  flex-shrink: 0;
}
.wm-empty {
  text-align: center;
  color: #94a3b8;
  padding: 20px;
  font-size: var(--fs-md);
}
</style>
