<template>
  <AppLayout>

    <div class="project-select">
      <div class="ps-header">
        <h1 class="ps-title">沧海API</h1>
        <p class="ps-subtitle">{{ t('app.selectProject') }}</p>
      </div>

      <div class="ps-body">
        <el-tabs v-model="activeTab" class="ps-tabs" @tab-change="onTabChange">
          <!-- 项目 -->
          <el-tab-pane label="项目" name="projects">
            <!-- 新建项目 -->
            <div class="ps-create">
              <el-input
                ref="newInputRef"
                v-model="newName"
                :placeholder="t('app.projectName')"
                size="large"
                @keyup.enter="confirmCreate"
              />
              <el-button type="primary" size="large" :icon="Plus" @click="confirmCreate">
                {{ t('app.createProject') }}
              </el-button>
            </div>

            <!-- 项目列表 -->
            <div v-if="projects.length" class="ps-grid">
              <div
                v-for="p in projects"
                :key="p.id"
                class="ps-card"
                @click="enter(p.id)"
              >
                <div class="ps-card-main">
                  <div class="ps-card-header">
                    <div class="ps-card-name" :title="p.name">{{ p.name }}</div>
                    <div class="ps-card-actions">
                      <el-tag v-if="p.currentUserRole" size="small" :type="roleTagType(p.currentUserRole)" class="ps-role-tag">
                        {{ roleLabel(p.currentUserRole) }}
                      </el-tag>
                      <el-button
                        v-if="canManage(p)"
                        size="small"
                        text
                        :icon="EditPen"
                        title="编辑"
                        @click.stop="openEdit(p)"
                      />
                      <el-button
                        size="small"
                        text
                        :icon="UserFilled"
                        title="成员管理"
                        @click.stop="openMembers(p)"
                      />
                      <el-button
                        size="small"
                        text
                        :icon="Star"
                        title="收藏（暂未开放）"
                        disabled
                        @click.stop
                      />
                      <el-button
                        v-if="canManage(p)"
                        size="small"
                        text
                        :icon="Delete"
                        title="删除"
                        @click.stop="confirmDeleteProject(p.id)"
                      />
                    </div>
                  </div>
                  <div class="ps-card-info">
                    <span>创建者：{{ p.createBy || '-' }}</span>
                    <span>最新更新：{{ formatTime(p.updateTime || p.createTime) }}</span>
                  </div>
                </div>
                <div class="ps-card-footer">
                  <div class="ps-card-count">接口数：{{ requestCount(p.id) }}</div>
                  <el-button v-if="canManage(p)" size="small" @click.stop="openEdit(p)">编辑</el-button>
                </div>
              </div>
            </div>

            <EmptyState v-else :title="t('app.noProject')" :icon="FolderOpened" />
          </el-tab-pane>

          <!-- 团队 -->
          <el-tab-pane label="团队" name="teams">
            <TeamManager ref="teamManagerRef" />
          </el-tab-pane>
        </el-tabs>
      </div>
  </div>

    <!-- 全局设置对话框 -->
    <el-dialog v-model="settingsVisible" title="全局设置" width="480px" :close-on-click-modal="false" destroy-on-close>
      <div class="settings-form">
        <div class="settings-item">
          <label>服务器地址</label>
          <el-input v-model="settingsForm.serverUrl" placeholder="http://127.0.0.1:8092" clearable />
          <span class="settings-hint">登录、同步等功能将使用此地址</span>
        </div>
        <div class="settings-item">
          <label>允许内网地址</label>
          <el-switch v-model="settingsForm.allowPrivateAddress" />
          <span class="settings-hint">开启后可访问 192.168.x.x / 172.16.x.x / 127.0.0.1 等内网地址（云元数据 169.254.169.254 始终拦截）</span>
        </div>
        <div class="settings-item">
          <label>当前状态</label>
          <div class="settings-status">
            <span>服务端连接：</span>
            <el-tag v-if="settingsOnline === null" size="small" type="info">未检测</el-tag>
            <el-tag v-else-if="settingsOnline" size="small" type="success">可连接</el-tag>
            <el-tag v-else size="small" type="danger">无法连接</el-tag>
            <el-button size="small" text @click="checkSettingsConnection" :loading="settingsChecking">检测</el-button>
          </div>
        </div>
      </div>
      <template #footer>
        <el-button @click="settingsVisible = false">取消</el-button>
        <el-button type="primary" @click="saveGlobalSettings">保存</el-button>
      </template>
    </el-dialog>

    <!-- 项目成员管理弹窗 -->
    <ProjectMemberManager v-model="memberDialogVisible" :project-id="memberProjectId" />

    <!-- 编辑项目弹窗 -->
    <el-dialog v-model="editDialogVisible" title="编辑项目" width="420px" :close-on-click-modal="false">
      <el-input v-model="editingProjectName" placeholder="项目名称" @keyup.enter="confirmEdit" />
      <template #footer>
        <el-button @click="editDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="confirmEdit">保存</el-button>
      </template>
    </el-dialog>
  </AppLayout>
</template>

<script setup lang="ts">
import EmptyState from '@/components/EmptyState.vue'
import { onMounted, reactive, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus, EditPen, Delete, UserFilled, Star, FolderOpened } from '@element-plus/icons-vue'
import { useProjects } from '@/composables/useProjects'
import { useSavedRequests } from '@/composables/useSavedRequests'
import { settings, saveSettings } from '@/composables/useSettings'
import { checkConnection, isLoggedIn } from '@/composables/useSync'
import { loadTeams } from '@/composables/useTeams'
import { useDataMode } from '@/composables/useDataMode'
import { i18n } from '@/i18n'
import AppLayout from '@/layouts/AppLayout.vue'
import TeamManager from '@/components/TeamManager.vue'
import ProjectMemberManager from '@/components/ProjectMemberManager.vue'
import type { Project } from '@/types'
import { APP_TZ_OFFSET_MS } from '@/utils'

const t = i18n.global.t
const router = useRouter()
const route = useRoute()

const { projects, addProject, deleteProject, updateProject, selectProject, selectedProjectId, consumeSuppressAutoEnter, load } = useProjects()
const { savedRequests, load: loadRequests } = useSavedRequests()
const { dataMode } = useDataMode()

// ====== 项目成员管理弹窗 ======
const memberProjectId = ref<string>('')
const memberDialogVisible = ref(false)

// 打开成员管理时同步更新全局选中角色，确保 ProjectMemberManager 的 canManage 判定正确
// （否则 selectedProjectRole 可能为 null，导致所有者/管理员看不到添加成员入口）
function openMembers(p: Project) {
  memberProjectId.value = p.id
  selectProject(p.id, p.currentUserRole)
  memberDialogVisible.value = true
}

// ====== 编辑项目弹窗 ======
const editingProjectId = ref<string>('')
const editingProjectName = ref('')
const editDialogVisible = ref(false)

// ====== 全局设置 ======
const settingsVisible = ref(false)
const settingsOnline = ref<boolean | null>(null)
const settingsChecking = ref(false)
const settingsForm = reactive({
  serverUrl: settings.serverUrl,
  allowPrivateAddress: settings.allowPrivateAddress,
})

async function checkSettingsConnection() {
  settingsChecking.value = true
  try {
    settingsOnline.value = await checkConnection(settingsForm.serverUrl)
  } catch {
    settingsOnline.value = false
  } finally {
    settingsChecking.value = false
  }
}

async function saveGlobalSettings() {
  await saveSettings({
    serverUrl: settingsForm.serverUrl.trim() || 'http://127.0.0.1:8092',
    allowPrivateAddress: settingsForm.allowPrivateAddress,
  })
  settingsVisible.value = false
  ElMessage.success('全局设置已保存')
} 

const newName = ref('')
const newInputRef = ref()

// ====== Tab 切换（项目 / 团队）======
const activeTab = ref<'projects' | 'teams'>('projects')
const teamManagerRef = ref<InstanceType<typeof TeamManager> | null>(null)

// 切换到「团队」tab 时刷新团队数据
function onTabChange(name: string | number) {
  if (name === 'teams') {
    teamManagerRef.value?.refresh()
  }
}

function requestCount(projectId: string): number {
  return savedRequests.value.filter(r => r.projectId === projectId).length
}

function formatTime(iso?: string): string {
  if (!iso) return '-'
  // 时间基准为 Asia/Shanghai：裸字符串按 +08:00 解析，并按同一基准渲染，
  // 这样展示值与数据库中存储的值一致，也不随查看者所在时区变化。
  const normalized = iso.includes('T') ? iso : iso.replace(' ', 'T')
  const d = new Date(/([zZ]|[+-]\d{2}:?\d{2})$/.test(normalized) ? normalized : normalized + '+08:00')
  if (Number.isNaN(d.getTime())) return iso
  const z = new Date(d.getTime() + APP_TZ_OFFSET_MS)
  const pad = (n: number) => n.toString().padStart(2, '0')
  return `${z.getUTCFullYear()}-${pad(z.getUTCMonth() + 1)}-${pad(z.getUTCDate())} ${pad(z.getUTCHours())}:${pad(z.getUTCMinutes())}`
}

function openEdit(p: Project) {
  editingProjectId.value = p.id
  editingProjectName.value = p.name
  editDialogVisible.value = true
}

async function confirmEdit() {
  const name = editingProjectName.value.trim()
  if (!name) {
    ElMessage.warning('请输入项目名称')
    return
  }
  const id = editingProjectId.value
  editDialogVisible.value = false
  try {
    await updateProject(id, name)
    ElMessage.success('已保存')
  } catch (e) {
    ElMessage.error('保存失败：' + (e instanceof Error ? e.message : String(e)))
  }
}

async function confirmDeleteProject(projectId: string) {
  try {
    await ElMessageBox.confirm(
      '确定要删除该项目吗？项目下的请求将一并删除。',
      t('app.delete') || '删除',
      { confirmButtonText: t('app.delete') || '删除', cancelButtonText: '取消', type: 'warning' },
    )
  } catch {
    return
  }
  try {
    await deleteProject(projectId)
    ElMessage.success('已删除')
  } catch (e) {
    ElMessage.error('删除失败：' + (e instanceof Error ? e.message : String(e)))
  }
}

function enter(projectId: string) {
  // 持久化选中项目及当前用户角色（写入 composable，自动存 localStorage），刷新后由路由自动恢复
  const p = projects.find(c => c.id === projectId)
  selectProject(projectId, p?.currentUserRole)
  router.push(`/project/${projectId}`)
}

/** 角色中文标签 */
function roleLabel(role?: string): string {
  switch (role) {
    case 'owner': return '所有者'
    case 'admin': return '管理员'
    case 'readwrite': return '读写'
    case 'readonly': return '只读'
    case 'inherit': return '继承团队权限'
    default: return role || ''
  }
}

/** 角色标签样式 */
function roleTagType(role?: string): 'primary' | 'success' | 'warning' | 'info' | 'danger' {
  switch (role) {
    case 'owner': return 'primary'
    case 'admin': return 'warning'
    case 'readwrite': return 'success'
    case 'readonly': return 'info'
    case 'inherit': return 'success'
    default: return 'info'
  }
}

/** 是否拥有管理权限（编辑/删除项目、管理成员）：仅所有者与管理员 */
function canManage(p: Project): boolean {
  return p.currentUserRole === 'owner' || p.currentUserRole === 'admin'
}

async function confirmCreate() {
  const name = newName.value.trim()
  if (!name) {
    ElMessage.warning(t('app.projectName'))
    return
  }
  if (dataMode.value === 'online' && !isLoggedIn.value) {
    ElMessage.warning('请先登录后再创建在线项目')
    return
  }
  try {
    const p = await addProject(name)
    newName.value = ''
    if (p) {
      ElMessage.success(t('app.createProject') + ' ✓')
      enter(p.id)
    }
  } catch (e) {
    ElMessage.error('创建失败：' + (e instanceof Error ? e.message : String(e)))
  }
}

onMounted(async () => {
  // 在线模式但未登录：项目/接口/团队均无云端数据可拉（get_projects、get_teams 必然 401），
  // 在线列表也已在登出时清空，故跳过本轮加载，避免无意义的 IPC 调用。
  // （离线模式无需登录，仍需加载本地数据。）
  if (dataMode.value === 'offline' || isLoggedIn.value) {
    // Phase 8.4：useSavedRequests 不再在 import 时自动加载（副作用已移除），
    // 本页需要 savedRequests 统计各项目接口数，因此在挂载时显式加载。
    await Promise.all([load(), loadTeams(), loadRequests()])
  }
  // 支持从调试页「团队管理」按钮跳转过来，直接激活团队 tab
  if (route.query.tab === 'teams') {
    activeTab.value = 'teams'
    teamManagerRef.value?.refresh()
  }
  // 若本次是“切换模式”回到项目页，则不自动进入上次项目，避免立即又跳回接口页。
  // 否则（如页面刷新）若之前已选过项目且仍存在，直接进入调试页以保持上下文。
  if (consumeSuppressAutoEnter()) return
  const saved = selectedProjectId.value
  if (saved && projects.some(p => p.id === saved)) {
    router.replace(`/project/${saved}`)
  }
})
</script>

<style scoped>
.project-select {
  height: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  background:
    radial-gradient(900px 340px at 50% -80px, rgba(99, 102, 241, 0.08), transparent 70%),
    var(--bg);
  overflow: auto;
}
.ps-header {
  margin-top: 8vh;
  text-align: center;
}
.ps-title {
  font-size: var(--fs-hero);
  font-weight: 700;
  color: var(--text-1);
  letter-spacing: 1px;
}
.ps-subtitle {
  margin-top: 8px;
  color: var(--text-3);
  font-size: var(--fs-xl);
}
.ps-body {
  width: min(880px, 92vw);
  margin-top: 32px;
}
.ps-create {
  display: flex;
  gap: 12px;
  margin-bottom: 24px;
}
.ps-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
  gap: 16px;
}
.ps-card {
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  background: var(--surface);
  border: 1px solid var(--border-soft);
  border-radius: var(--r-md);
  padding: 16px;
  cursor: pointer;
  box-shadow: var(--shadow-1);
  transition: border-color var(--duration-fast) var(--ease-out),
    box-shadow var(--duration-fast) var(--ease-out),
    transform var(--duration-fast) var(--ease-out);
  min-height: 110px;
}
.ps-card:hover {
  border-color: var(--brand-1);
  box-shadow: var(--shadow-2);
  transform: translateY(-2px);
}
.ps-card-main {
  flex: 1;
}
.ps-card-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}
.ps-card-name {
  flex: 1;
  font-size: var(--fs-2xl);
  font-weight: 600;
  color: var(--text-1);
  word-break: break-all;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.ps-card-actions {
  display: flex;
  gap: 2px;
  flex-shrink: 0;
}
.ps-card-actions .el-button {
  margin: 0;
  padding: 4px;
  color: var(--text-4);
}
.ps-card-actions .el-button:hover {
  color: var(--brand-1);
}
.ps-card-info {
  margin-top: 10px;
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: var(--fs-sm);
  color: var(--text-3);
}
.ps-card-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-top: 14px;
}
.ps-card-count {
  font-size: var(--fs-md);
  color: var(--text-3);
}
/* ====== Tabs ====== */
.ps-tabs {
  width: 100%;
}
.ps-tabs :deep(.el-tabs__header) {
  margin-bottom: 20px;
}
.ps-tabs :deep(.el-tabs__item) {
  font-size: var(--fs-xl);
  font-weight: 600;
}

/* ====== Global Settings Dialog ====== */
.settings-form .settings-item {
  margin-bottom: 18px;
}
.settings-form .settings-item label {
  display: block;
  font-weight: 600;
  font-size: var(--fs-md);
  color: var(--text-1);
  margin-bottom: 6px;
}
.settings-form .settings-hint {
  display: block;
  font-size: var(--fs-xs);
  color: var(--text-4);
  margin-top: 4px;
}
.settings-form .settings-status {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: var(--fs-md);
  color: var(--text-2);
}
</style>
