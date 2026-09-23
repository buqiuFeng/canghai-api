<template>
  <el-dialog
    v-model="visible"
    title="项目成员管理"
    width="560px"
    :close-on-click-modal="false"
    destroy-on-close
    @open="onOpen"
  >
    <div class="pm-body">
      <!-- 添加成员（仅所有者 / 管理员） -->
      <div v-if="canManage" class="pm-add-row">
        <el-select v-model="inviteType" size="small" style="width: 90px">
          <el-option label="用户" value="user" />
          <el-option label="团队" value="team" />
        </el-select>

        <template v-if="inviteType === 'team'">
          <el-select
            v-model="inviteTeamId"
            size="small"
            placeholder="选择团队"
            filterable
            style="flex: 1"
          >
            <el-option
              v-for="t in teams"
              :key="t.id"
              :label="t.name"
              :value="t.id"
            />
          </el-select>
          <span class="pm-inherit-hint">继承团队权限</span>
        </template>

        <template v-else>
          <el-select v-model="inviteRole" size="small" style="width: 100px">
            <el-option label="管理员" value="admin" />
            <el-option label="读写" value="readwrite" />
            <el-option label="只读" value="readonly" />
          </el-select>
          <el-input
            v-model="inviteUsername"
            size="small"
            placeholder="输入用户名"
            clearable
            style="flex: 1"
            @keyup.enter="doInvite"
          />
        </template>

        <el-button size="small" type="primary" :loading="inviting" @click="doInvite">
          添加
        </el-button>
      </div>

      <!-- 成员列表 -->
      <el-table v-loading="loading" :data="members" size="small" style="width: 100%">
        <el-table-column label="成员" min-width="160">
          <template #default="{ row }">
            <div class="pm-member">
              <el-icon><UserFilled /></el-icon>
              <span>{{ row.memberName || row.memberId }}</span>
              <el-tag v-if="row.memberType === 'team'" size="small" type="info">团队</el-tag>
            </div>
          </template>
        </el-table-column>
        <el-table-column label="角色" width="120">
          <template #default="{ row }">
            <el-tag v-if="row.role === 'inherit'" size="small" type="success">继承团队权限</el-tag>
            <template v-else>{{ ROLE_LABEL[row.role] || row.role }}</template>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="120" align="right">
          <template #default="{ row }">
            <!-- 当前用户自己：仅 user 类型成员可退出（团队关联不可由个人退出） -->
            <el-button
              v-if="isSelf(row)"
              size="small"
              link
              type="warning"
              :loading="removingId === row.memberId"
              @click="doQuit(row)"
            >
              退出
            </el-button>
            <!-- 移除他人：仅所有者 / 管理员可见 -->
            <el-button
              v-else-if="canManage"
              size="small"
              link
              type="danger"
              :loading="removingId === row.memberId"
              @click="doRemove(row)"
            >
              <el-icon><Delete /></el-icon>
            </el-button>
          </template>
        </el-table-column>
      </el-table>

      <EmptyState v-if="members.length === 0 && !loading" compact title="暂无成员" />
    </div>

    <template #footer>
      <el-button size="small" @click="visible = false">关闭</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import EmptyState from '@/components/EmptyState.vue'
import { ref, computed } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { UserFilled, Delete } from '@element-plus/icons-vue'
import { useProjectMembers } from '@/composables/useProjectMembers'
import { useTeams } from '@/composables/useTeams'
import { currentUser } from '@/composables/useSync'
import { selectedProjectRole } from '@/composables/useProjects'
import type { ProjectMemberInfo } from '@/types'

const props = defineProps<{
  projectId?: string
}>()

const visible = defineModel<boolean>({ default: false })
const { members, loading, load, addMember, removeMember } = useProjectMembers()
const { teams, loadTeams } = useTeams()

// 当前用户在该项目中是否为所有者或管理员：仅其可移除其他成员
const canManage = computed(() => {
  const r = selectedProjectRole.value
  return r === 'owner' || r === 'admin'
})

// 该行是否为当前登录用户自己（user 类型成员，且 member_id 对应当前用户 id）
function isSelf(row: ProjectMemberInfo): boolean {
  return row.memberType === 'user' && !!currentUser.value && row.memberId === currentUser.value.id
}

const inviteType = ref<'user' | 'team'>('user')
const inviteUsername = ref('')
const inviteRole = ref<ProjectMemberInfo['role']>('readwrite')
const inviteTeamId = ref('')
const inviting = ref(false)
const removingId = ref<string | null>(null)

const ROLE_LABEL: Record<string, string> = {
  owner: '所有者',
  admin: '管理员',
  readwrite: '读写',
  readonly: '只读',
}

async function onOpen() {
  if (!props.projectId) return
  await Promise.all([load(props.projectId), loadTeams()])
}

async function doInvite() {
  if (!props.projectId) return
  if (inviteType.value === 'team') {
    if (!inviteTeamId.value) {
      ElMessage.warning('请选择团队')
      return
    }
  } else {
    const name = inviteUsername.value.trim()
    if (!name) {
      ElMessage.warning('请输入用户名')
      return
    }
  }
  inviting.value = true
  try {
    if (inviteType.value === 'team') {
      await addMember(props.projectId, inviteTeamId.value, inviteRole.value, 'team')
    } else {
      await addMember(props.projectId, inviteUsername.value.trim(), inviteRole.value, 'user')
    }
    ElMessage.success('添加成功')
    inviteUsername.value = ''
    inviteTeamId.value = ''
  } catch (e) {
    ElMessage.error('添加失败：' + String(e))
  } finally {
    inviting.value = false
  }
}

/** 退出项目：当前用户移除自己的成员关系（仅 user 类型成员可退出） */
async function doQuit(row: ProjectMemberInfo) {
  if (!props.projectId) return
  try {
    await ElMessageBox.confirm(
      `确定退出项目吗？退出后将不再能看到该项目。`,
      '退出项目',
      { confirmButtonText: '退出', cancelButtonText: '取消', type: 'warning' }
    )
  } catch {
    return
  }
  removingId.value = row.memberId
  try {
    await removeMember(props.projectId, row.memberType, row.memberId)
    ElMessage.success('已退出项目')
  } catch (e) {
    ElMessage.error('退出失败：' + String(e))
  } finally {
    removingId.value = null
  }
}

async function doRemove(row: ProjectMemberInfo) {
  if (!props.projectId) return
  try {
    await ElMessageBox.confirm(
      `确定移除成员 "${row.memberName || row.memberId}" 吗？`,
      '移除成员',
      { confirmButtonText: '移除', cancelButtonText: '取消', type: 'warning' }
    )
  } catch {
    return
  }
  removingId.value = row.memberId
  try {
    await removeMember(props.projectId, row.memberType, row.memberId)
    ElMessage.success('已移除')
  } catch (e) {
    ElMessage.error('移除失败：' + String(e))
  } finally {
    removingId.value = null
  }
}
</script>

<style scoped>
.pm-body {
  display: flex;
  flex-direction: column;
  gap: 16px;
}
.pm-add-row {
  display: flex;
  gap: 8px;
  align-items: center;
}
.pm-member {
  display: flex;
  align-items: center;
  gap: 6px;
}
.pm-inherit-hint {
  font-size: var(--fs-sm);
  color: #67c23a;
  white-space: nowrap;
}
.pm-empty {
  text-align: center;
  color: #999;
  padding: 24px 0;
  font-size: var(--fs-md);
}
</style>
