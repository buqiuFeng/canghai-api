<template>
  <div class="app-shell">
    <header class="topbar">
      <!-- 品牌区（全局公共） -->
      <div class="brand">
        <div class="brand-badge">
          <el-icon><Promotion /></el-icon>
        </div>
        <div>
          <div class="brand-title">沧海·API 调试工具</div>
          <div class="brand-sub">轻量级 HTTP 调试 · Tauri 后端代理无 CORS 限制</div>
        </div>
      </div>

      <!-- 页面特有控件（由各页面通过 #header 插槽注入） -->
      <slot name="header" />

      <!-- 右上角全局操作：全局设置 / 在线离线 / 登录同步 / 语言 / 审计 -->
      <div class="topbar-right topbar-global">
        <!-- 登录 / 同步（仅在线模式显示） -->
        <div class="topbar-group" v-if="!isOffline">
          <el-button v-if="!isLoggedIn" size="small" plain type="primary" :icon="User" @click="openLoginDialog">
            登录
          </el-button>
          <el-popover
              v-else
              placement="bottom"
              :width="200"
              trigger="click"
          >
            <template #reference>
              <el-button size="small" plain type="success" :icon="UserFilled">
                {{ currentUser?.nickname || currentUser?.username || '用户' }}
                <span class="dot online" style="margin-left: 4px;"></span>
              </el-button>
            </template>
            <div class="user-popover">
              <div class="user-info-line">
                <el-icon><UserFilled /></el-icon>
                <span>{{ currentUser?.nickname || currentUser?.username }}</span>
              </div>
              <div v-if="currentUser?.username" class="user-info-line sub">@{{ currentUser.username }}</div>
              <div v-if="currentUser?.email" class="user-info-line sub">{{ currentUser.email }}</div>
              <el-divider style="margin: 8px 0;" />
              <el-button size="small" text type="danger" @click="doLogout" style="width:100%">
                退出登录
              </el-button>
            </div>
          </el-popover>
          <el-popover
              placement="bottom"
              :width="320"
              trigger="click"
              :visible="syncPopVisible"
              @update:visible="(v: boolean) => syncPopVisible = v"
          >
            <template #reference>
              <el-button
                  size="small"
                  plain
                  :type="syncOnline ? 'default' : 'warning'"
                  :class="['btn-sync', { 'syncing': syncStatus === 'syncing' }]"
                  :loading="syncStatus === 'syncing'"
                  :icon="Cloudy"
              >
                同步
                <span v-if="syncOnline" class="dot online"></span>
                <span v-else class="dot offline"></span>
              </el-button>
            </template>
            <div class="sync-popover">
              <div class="sync-title">云端同步设置</div>
              <div v-if="!isLoggedIn" class="sync-warning">
                <el-icon><WarningFilled /></el-icon>
                请先登录后再同步
              </div>
              <div class="sync-row">
                <label>服务器</label>
                <code class="sync-server-addr">{{ settings.serverUrl }}</code>
              </div>
              <div class="sync-status-line">
                状态：
                <el-tag v-if="syncOnline" size="small" type="success">已连接</el-tag>
                <el-tag v-else size="small" type="warning">未连接</el-tag>
                <el-tag v-if="isLoggedIn" size="small" type="success" style="margin-left:4px">已登录</el-tag>
                <el-tag v-else size="small" type="info" style="margin-left:4px">未登录</el-tag>
              </div>
              <div v-if="syncMessage" class="sync-msg">{{ syncMessage }}</div>
              <div class="sync-actions">

                <el-button size="small" type="primary" @click="doSync" :loading="syncStatus === 'syncing'" :disabled="!isLoggedIn">
                  立即同步
                </el-button>
              </div>
            </div>
          </el-popover>
        </div>
        <!-- 主题切换（浅色 / 暗色） -->
        <el-tooltip :content="theme === 'dark' ? '切换为浅色模式' : '切换为深色模式'" placement="bottom">
          <el-button size="small" plain :icon="theme === 'dark' ? Sunny : Moon" @click="toggleTheme" />
        </el-tooltip>

        <!-- 全局设置 -->
        <el-tooltip :content="t('common.settings')" placement="bottom">
          <el-button size="small" plain :icon="Setting" @click="openSettings" />
        </el-tooltip>

        <!-- 在线/离线切换 -->
        <div class="topbar-group">
          <el-tooltip
            :content="dataMode === 'offline' ? '当前：离线模式（数据存于本地独立数据库，与在线数据隔离）' : '当前：在线模式（数据存于本地数据库，可被云同步）'"
            placement="bottom"
          >
            <el-switch
              v-model="isOffline"
              inline-prompt
              active-text="离线"
              inactive-text="在线"
              active-color="#e6a23c"
              inactive-color="#409eff"
              @change="onDataModeChange"
            />
          </el-tooltip>
        </div>



        <el-dropdown trigger="click" @command="onChangeLang">
          <el-button size="small" plain :icon="Document" title="语言 / Language">{{ currentLangLabel }}</el-button>
          <template #dropdown>
            <el-dropdown-menu>
              <el-dropdown-item
                v-for="opt in langOptions"
                :key="opt.value"
                :command="opt.value"
                :class="{ 'lang-active': opt.value === currentLang }"
              >
                {{ opt.label }}
              </el-dropdown-item>
            </el-dropdown-menu>
          </template>
        </el-dropdown>
        <el-button size="small" plain :icon="Document" title="操作审计日志" @click="auditDrawerVisible = true">
          审计
        </el-button>
      </div>
    </header>

    <!-- 页面主体 -->
    <main class="content">
      <slot />
    </main>

    <!-- 全局审计抽屉 -->
    <AuditLogDrawer v-model:visible="auditDrawerVisible" request-id="" />

    <!-- 全局编辑冲突对比框 -->
    <ConflictCompareDialog />

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
        <div v-if="isLoggedIn" class="settings-item">
          <label>登录账号</label>
          <span class="settings-value">{{ currentUser?.nickname || currentUser?.username }} <el-tag size="small" type="success">已登录</el-tag></span>
        </div>
      </div>
      <template #footer>
        <el-button @click="settingsVisible = false">取消</el-button>
        <el-button type="primary" @click="saveGlobalSettings">保存</el-button>
      </template>
    </el-dialog>

    <!-- 登录对话框 -->
    <el-dialog v-model="loginDialogVisible" :title="authMode === 'login' ? '登录' : '注册'" width="400px" :close-on-click-modal="false" destroy-on-close>
      <div class="auth-form">
        <div class="auth-row">
          <label>用户名</label>
          <el-input v-model="authForm.username" placeholder="输入用户名" />
        </div>
        <div v-if="authMode === 'register'" class="auth-row">
          <label>昵称</label>
          <el-input v-model="authForm.nickname" placeholder="给自己起个名字（选填）" />
        </div>
        <div class="auth-row">
          <label>密码</label>
          <el-input v-model="authForm.password" type="password" placeholder="输入密码" show-password />
        </div>
        <div v-if="authMode === 'register'" class="auth-row">
          <label>邮箱</label>
          <el-input v-model="authForm.email" placeholder="选填" />
        </div>
        <div v-if="authError" class="auth-error">{{ authError }}</div>
        <div class="auth-actions">
          <el-button link type="primary" @click="authMode = authMode === 'login' ? 'register' : 'login'">
            {{ authMode === 'login' ? '没有账号？注册' : '已有账号？登录' }}
          </el-button>
        </div>
      </div>
      <template #footer>
        <el-button @click="loginDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="doAuth" :loading="authLoading">
          {{ authMode === 'login' ? '登录' : '注册' }}
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, reactive, ref } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage } from 'element-plus'
import { Promotion, Document, Setting, User, UserFilled, Cloudy, WarningFilled, Sunny, Moon } from '@element-plus/icons-vue'
import { useI18n } from 'vue-i18n'
import { setLang, langOptions } from '@/i18n'
import AuditLogDrawer from '@/components/AuditLogDrawer.vue'
import ConflictCompareDialog from '@/components/ConflictCompareDialog.vue'
import { useDataMode } from '@/composables/useDataMode'
import { useTheme } from '@/composables/useTheme'
import { settings, loadSettings, saveSettings } from '@/composables/useSettings'
import { loadTeams, useTeams } from '@/composables/useTeams'
import { useProjects } from '@/composables/useProjects'
import {
  syncStatus, syncMessage, syncOnline, currentUser, isLoggedIn,
  currentTeamId,
  startAutoSync, stopAutoSync, getSyncConfig, setSyncConfig, sync, syncConfigLoaded,
  login, register, logout, checkConnection,
} from '@/composables/useSync'
import type { SyncConfig } from '@/composables/useSync'

const { t, locale } = useI18n()

/* ============ 路由 ============ */
// 登出后需跳回首页（项目选择页），故此处显式取 router
const router = useRouter()

const currentLang = locale
const currentLangLabel = computed(
  () => langOptions.find((o) => o.value === currentLang.value)?.label ?? '简体中文',
)
function onChangeLang(lang: string) {
  setLang(lang as any)
  ElMessage.success(t('common.ok'))
}

const auditDrawerVisible = ref(false)

/* ============ 主题（浅色 / 暗色） ============ */
const { theme, toggleTheme } = useTheme()

/* ============ 数据模式（在线/离线隔离） ============ */
const { dataMode, setMode } = useDataMode()
const isOffline = computed({
  get: () => dataMode.value === 'offline',
  set: (v: boolean) => {
    const target = v ? 'offline' : 'online'
    // 切换到在线模式必须已登录，否则拦截并保持离线、提示登录
    if (target === 'online' && !isLoggedIn.value) {
      ElMessage.warning('在线模式需要先登录')
      openLoginDialog()
      return
    }
    setMode(target)
  },
})
async function onDataModeChange() {
  // 兜底：若以其他方式进入在线模式但未登录，回退到离线
  if (dataMode.value === 'online' && !isLoggedIn.value) {
    ElMessage.warning('在线模式需要先登录')
    openLoginDialog()
    setMode('offline')
    return
  }
  ElMessage.success(dataMode.value === 'offline' ? '已切换到离线模式（本地独立数据库）' : '已切换到在线模式')
  // AppLayout 仅负责刷新团队与项目信息；接口/环境/分类等其他数据由各页面对应 composable 在其挂载或 selectedProjectId 变化时自行加载，不在此处处理。
  // 离线模式下云端不可达，跳过 loadTeams，直接加载本地项目。
  if (dataMode.value === 'online') {
    await useTeams().loadTeams().catch(() => {})
  }
  await Promise.all([
    useProjects().load()
  ])
}

/* ============ 认证 ============ */
const loginDialogVisible = ref(false)
const authMode = ref<'login' | 'register'>('login')
const authLoading = ref(false)
const authError = ref('')
const authForm = reactive({
  username: '',
  nickname: '',
  password: '',
  email: '',
})

function openLoginDialog() {
  authMode.value = 'login'
  authError.value = ''
  authForm.username = ''
  authForm.nickname = ''
  authForm.password = ''
  authForm.email = ''
  loginDialogVisible.value = true
}

async function doAuth() {
  authError.value = ''
  if (!authForm.username.trim()) {
    authError.value = '请输入用户名'
    return
  }
  if (!authForm.password || authForm.password.length < 6) {
    authError.value = '密码长度不能少于 6 位'
    return
  }
  if (!settings.serverUrl.trim()) {
    authError.value = '请先在全局设置中配置服务器地址'
    return
  }
  authLoading.value = true
  try {
    if (authMode.value === 'login') {
      await login(settings.serverUrl, authForm.username.trim(), authForm.password)
    } else {
      const nickname = authForm.nickname.trim() || authForm.username.trim()
      await register(settings.serverUrl, authForm.username.trim(), authForm.password, nickname, authForm.email.trim())
    }
    ElMessage({ message: authMode.value === 'login' ? '登录成功' : '注册成功', type: 'success', duration: 15000, showClose: true })
    loginDialogVisible.value = false
    // 加载团队列表
    await loadTeams()
    // 登录成功后，若当前仍处于离线模式，自动切换到在线模式
    // （在线模式需先登录才能使用，登录即满足前置条件），
    // 避免“登录后仍停留在离线模式、显示离线项目”的问题。
    if (dataMode.value === 'offline') {
      setMode('online')
      await Promise.all([
        useProjects().load(),
      ])
    } else {
      // 已在线：团队列表与项目列表刷新；接口/环境/分类等其他数据由各页面对应 composable 自行加载，不在 AppLayout 中处理
      await useProjects().load()
    }
    // 保存配置（使用当前激活团队，避免把登录时正确的 teamId 覆盖成 'default'）。
    // 令牌已由 login() 内的 persistAuth 写入，这里不再重复上报，避免「用内存态覆盖登录态」。
    syncConfig.serverUrl = settings.serverUrl
    await setSyncConfig({ serverUrl: settings.serverUrl, teamId: currentTeamId.value })
    syncOnline.value = await checkConnection(settings.serverUrl)
  } catch (e: any) {
    authError.value = typeof e === 'string' ? e : (e?.message || '操作失败')
  } finally {
    authLoading.value = false
  }
}

async function doLogout() {
  try {
    await logout()
    // 登出后项目上下文已失效（在线项目列表会被清空）：回到首页（项目选择页），
    // 否则会停留在 /project/:id 上展示已清空的在线数据。
    // 显式清除选中项目，避免 ProjectSelectView 挂载时按「上次项目」又自动跳回调试页。
    useProjects().clearProject()
    router.replace('/')
    ElMessage.success('已退出登录')
  } catch (e: any) {
    ElMessage.error('退出失败: ' + (e?.message || e))
  }
}

/* ============ 全局设置 ============ */
const settingsVisible = ref(false)
const settingsOnline = ref<boolean | null>(null)
const settingsChecking = ref(false)
const settingsForm = reactive({
  serverUrl: settings.serverUrl,
  allowPrivateAddress: settings.allowPrivateAddress,
})

function openSettings() {
  settingsForm.serverUrl = settings.serverUrl
  settingsForm.allowPrivateAddress = settings.allowPrivateAddress
  settingsVisible.value = true
}

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
  syncConfig.serverUrl = settings.serverUrl
  settingsVisible.value = false
  // 重新检测连通性
  syncOnline.value = await checkConnection(settings.serverUrl)
  ElMessage.success('全局设置已保存')
}

/* ============ 同步 ============ */
const syncPopVisible = ref(false)
const syncConfig = reactive<SyncConfig>({ serverUrl: settings.serverUrl, teamId: 'default' })

async function loadSyncConfig() {
  try {
    // 复用启动期 `initAuth` 读到的持久化配置（见 `stores/session.syncServerUrl`）。
    // 改前这里是「loadSettings() 读一次 get_sync_config + getSyncConfig() 再读一次」，
    // 叠加 App.vue 的 initAuth，刷新一次页面共发 3 次同一条命令的 IPC。
    await loadSettings()
    // 兜底：仅当启动期压根没读成功过配置时才补读一次（保持原有自愈能力）；
    // 读取成功但未登录属于正常状态，不再多打一次 IPC。
    if (!syncConfigLoaded.value) {
      await getSyncConfig()
    }
    // 以全局设置为准
    syncConfig.serverUrl = settings.serverUrl
    syncConfig.teamId = currentTeamId.value || 'default'
  } catch { /* use defaults */ }
}

async function doSync() {
  if (!isLoggedIn.value) {
    ElMessage.warning('请先登录后再同步')
    return
  }
  try {
    await sync({ serverUrl: settings.serverUrl, teamId: currentTeamId.value })
    ElMessage.success('同步完成')
    syncPopVisible.value = false
    // 刷新本地共享数据（请求、环境变量）；分类按项目刷新，由各页面自行处理

  } catch (e: any) {
    ElMessage.error('同步失败: ' + (e?.message || e))
  }
}

/* ============ 初始化 ============ */
onMounted(async () => {
  // 认证状态由 App.vue 统一在渲染路由前初始化（authReady 门控），这里不再调用 initAuth，
  // 避免重复初始化。currentUser 此时已就绪。
  // 团队列表由 useTeams / ProjectSelectView 等各自按需加载，AppLayout 不再统一调用 loadTeams()，
  // 避免与各 composable 的 onMounted / useModeReload 重复请求 get_teams。
  // 加载同步配置并启动定时同步（startAutoSync 内部会自行读取 sync config）
  await loadSyncConfig()
  startAutoSync(60000) // 60 秒检测一次

})


onUnmounted(() => {
  stopAutoSync()
})
</script>

<style scoped>
/* ====== 公共布局：外壳 + 顶栏（收敛到本组件作用域，避免与外层视图的全局类名相互覆盖） ====== */
.app-shell {
  display: flex;
  flex-direction: column;
  height: 100vh;
  overflow: hidden;
}

.topbar {
  flex: 0 0 auto;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  row-gap: 6px;
  min-height: var(--topbar-h);
  padding: 6px 16px;
  background: var(--surface);
  border-bottom: 1px solid var(--border-soft);
  box-shadow: var(--shadow-1);
  flex-wrap: wrap;
  z-index: 10;
}

.brand {
  display: flex;
  align-items: center;
  gap: 12px;
}

.brand-badge {
  width: 32px;
  height: 32px;
  border-radius: 9px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  background: var(--brand-gradient);
  color: #fff;
  font-size: var(--fs-2xl);
  flex-shrink: 0;
  box-shadow: var(--shadow-brand);
}

.brand-title {
  font-size: var(--fs-xl);
  font-weight: 700;
  color: var(--text-1);
  line-height: 1.2;
  letter-spacing: -0.02em;
  white-space: nowrap;
}

.brand-sub {
  font-size: var(--fs-2xs);
  color: var(--text-3);
  margin-top: 1px;
  white-space: nowrap;
}
@media (max-width: 1280px) {
  .brand-sub { display: none; }
}

.topbar-right {
  display: flex;
  align-items: center;
  gap: 10px;
}

.topbar-global {
  margin-left: auto;
}

.topbar-sep {
  width: 1px;
  height: 20px;
  background: var(--border);
}

.topbar-group {
  display: flex;
  align-items: center;
  gap: 8px;
}

.content {
  flex: 1 1 auto;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

/* ====== 公共头部：登录/同步弹窗 ====== */
.dot {
  display: inline-block;
  width: 7px;
  height: 7px;
  border-radius: 50%;
}
.dot.online {
  background: #67c23a;
  box-shadow: 0 0 4px rgba(103, 194, 58, 0.6);
}
.dot.offline {
  background: #e6a23c;
  box-shadow: 0 0 4px rgba(230, 162, 60, 0.6);
}

.user-popover .user-info-line {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: var(--fs-lg);
  color: var(--text);
}
.user-popover .user-info-line.sub {
  font-size: var(--fs-sm);
  color: var(--text-muted);
  margin-left: 22px;
}

.sync-popover .sync-title {
  font-weight: 600;
  margin-bottom: 8px;
  color: var(--text);
}
.sync-popover .sync-warning {
  display: flex;
  align-items: center;
  gap: 6px;
  color: #e6a23c;
  font-size: var(--fs-md);
  margin-bottom: 8px;
}
.sync-popover .sync-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: var(--fs-md);
  margin: 6px 0;
  color: var(--text);
}
.sync-popover .sync-row label {
  color: var(--text-muted);
}
.sync-popover .sync-server-addr {
  font-size: var(--fs-xs);
  max-width: 200px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.sync-popover .sync-status-line {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: var(--fs-md);
  margin: 6px 0;
  color: var(--text-muted);
}
.sync-popover .sync-msg {
  font-size: var(--fs-sm);
  color: var(--text-muted);
  margin: 6px 0;
  word-break: break-all;
}
.sync-popover .sync-actions {
  display: flex;
  gap: 8px;
  margin-top: 10px;
}

.settings-form .settings-item {
  margin-bottom: 16px;
}
.settings-form .settings-item label {
  display: block;
  font-size: var(--fs-md);
  color: var(--text-muted);
  margin-bottom: 6px;
}
.settings-form .settings-hint {
  display: block;
  font-size: var(--fs-sm);
  color: var(--text-muted);
  margin-top: 6px;
}
.settings-form .settings-status {
  display: flex;
  align-items: center;
  gap: 8px;
}
.settings-form .settings-value {
  display: flex;
  align-items: center;
  gap: 8px;
}
</style>
