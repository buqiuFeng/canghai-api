import { reactive, ref, computed } from 'vue'
import { useDataMode } from './useDataMode'
import { teams, loadTeams } from './useTeams'
import { currentUser, isLoggedIn } from './useSync'
import { useModeReload, modeAtCall, modeStillValid } from './useModeReload'
import type { Project } from '@/types'
import { now } from '@/utils'
import { useProjectRepo, type RawProject } from '@/repositories/projectRepo'

const { dataMode } = useDataMode()
const repo = useProjectRepo()

function uid(): string {
  return 'p_' + Math.random().toString(36).slice(2, 10) + Date.now().toString(36)
}

// 项目：扁平结构，无多级嵌套
const projects = reactive<Project[]>([])

// 在线/离线两套独立的项目选择状态（物理库已隔离，这里做逻辑隔离）：
// 切换数据模式后各自恢复自己模式下的选中项，互不串扰。
function storageKey(mode: string) {
  return `canghaiApi.selectedProjectId.${mode}`
}
function readSelected(mode: string): string | null {
  try {
    return localStorage.getItem(storageKey(mode)) || null
  } catch {
    return null
  }
}
function writeSelected(mode: string, id: string | null) {
  try {
    if (id) localStorage.setItem(storageKey(mode), id)
    else localStorage.removeItem(storageKey(mode))
  } catch { /* ignore */ }
}

// 当前模式下的选中项目 ID（响应式跟随 dataMode）
const selectedProjectId = computed<string | null>({
  get: () => readSelected(dataMode.value),
  set: (id) => writeSelected(dataMode.value, id),
})

/** 设置当前模式下的项目并持久化，刷新后自动恢复 */
function selectProject(id: string | null, role?: Project['currentUserRole']) {
  selectedProjectId.value = id
  selectedProjectRole.value = role ?? null
}

// 当前选中项目内“当前用户的角色”，供详情页按权限控制（与 projectId 一同按模式隔离）
export const selectedProjectRole = ref<Project['currentUserRole'] | null>(null)

/** 清除当前模式下的项目选择（如切换项目回到选择页） */
function clearProject() {
  selectProject(null)
}

// 切换数据模式后回到项目页时，抑制 ProjectSelectView 的“自动进入上次项目”逻辑，
// 避免刚回到项目页又被自动跳回接口页。一次性消费。
const suppressAutoEnter = ref(false)
function consumeSuppressAutoEnter(): boolean {
  const v = suppressAutoEnter.value
  suppressAutoEnter.value = false
  return v
}

// 切换在线/离线、或登录/登出时：用当前模式物理库重新拉取项目，并校验选中项是否仍有效。
// 双 watch 与选中项失效校验已收敛到 useModeReload，避免各实体重复实现且行为不一致。
useModeReload(load, {
  // 登出时立即清空列表，避免旧在线项目残留（随后 load 在线模式查空，UI 看不到任何在线项目）
  onBeforeReload: () => {
    if (!isLoggedIn.value) {
      projects.splice(0, projects.length)
      clearProject()
    }
  },
  selectedId: () => selectedProjectId.value,
  onInvalidate: () => {
    const id = selectedProjectId.value
    if (id && !projects.some(p => p.id === id)) clearProject()
  },
})

function mapRow(r: RawProject): Project {
  return {
    id: r.id,
    name: r.name,
    parentId: r.parentId ?? null,
    sortOrder: r.sortOrder ?? 0,
    expanded: r.expanded ?? false,
    userId: r.userId ?? '',
    createBy: r.createBy ?? '',
    updateBy: r.updateBy ?? '',
    createTime: r.createTime,
    updateTime: r.updateTime,
    currentUserRole: r.currentUserRole as Project['currentUserRole'],
  }
}

async function load() {
  // 在线模式但未登录：无 token 时 Rust 侧 get_projects 直接返回 401（不会打到后端），
  // 且登出时在线项目已被清空，此处提前返回，避免未登录进入项目页时产生无意义调用。
  if (dataMode.value === 'online' && !isLoggedIn.value) return
  // 记录本次请求发起时的数据模式，防止切换模式时的并发请求互相串扰：
  // 例如先发离线请求、后发在线请求，若离线响应后到，会把离线项目写进在线列表。
  const captured = modeAtCall()
  try {
    // 在线模式下，项目按 team_id 过滤。若团队上下文尚未恢复（teams 为空），
    // 先恢复真实团队再查，否则 activeTeamId 可能仍是旧值导致查不到在线项目。
    if (captured === 'online' && teams.value.length === 0) {
      await loadTeams().catch(() => {})
    }
    // 在线模式下由 Rust 命令从当前登录 token 推导 user_id 过滤，前端无需再传 userId。
    const rows = await repo.fetchLocal()
    // 若期间数据模式已切换，丢弃过期结果，避免把另一模式的数据写入当前列表
    if (!modeStillValid(captured)) return
    if (!rows) return
    projects.splice(0, projects.length, ...rows.map(mapRow))
  } catch { /* ignore */ }
}

/**
 * 新增项目：先同步调用后端（Rust 命令 save_project → 在线模式再转发后端 /api/v1/project/save）
 * 保存入库，成功后再更新本地响应式状态；后端失败则抛错由调用方提示，避免“假成功”。
 */
async function addProject(name: string): Promise<Project | null> {
  // 在线模式下必须已登录才能创建项目（项目绑定用户），未登录则拒绝
  if (dataMode.value === 'online' && !isLoggedIn.value) {
    throw new Error('请先登录后再创建在线项目')
  }
  const maxOrder = Math.max(0, ...projects.map(c => c.sortOrder))
  // 在线模式下，新建项目默认把当前登录人作为项目所有者（create_by）
  const owner = dataMode.value === 'online'
    ? (currentUser.value?.username || currentUser.value?.id || '')
    : 'local'
  // 统一时间格式为 `YYYY-MM-DD HH:mm:ss`(Asia/Shanghai)，与其它实体及服务端 Java `now()` 保持一致
  const currentTime = now()
  const p: Project = {
    id: uid(),
    name,
    parentId: null,
    sortOrder: maxOrder + 1,
    expanded: true,
    userId: dataMode.value === 'online' ? (currentUser.value?.id ?? '') : '',
    createBy: owner,
    updateBy: owner,
    createTime: currentTime,
    updateTime: currentTime,
    currentUserRole: 'owner',
  }
  // 等待后端保存成功，失败抛错（不更新本地，保持与后端一致）
  await repo.persist(p)
  projects.push(p)
  return p
}

/**
 * 编辑项目：先同步调用后端（update_project → 在线模式转发后端 /api/v1/project/save），
 * 成功后再更新本地响应式状态；失败抛错由调用方提示。
 */
async function updateProject(id: string, name: string) {
  const p = projects.find(c => c.id === id)
  // 先同步后端，失败抛错，不改动本地
  await repo.update(id, name, p?.updateBy || currentUser.value?.username || currentUser.value?.id || 'local')
  if (p) {
    p.name = name
    p.updateBy = currentUser.value?.username || currentUser.value?.id || 'local'
    p.updateTime = now()
  }
}

/**
 * 删除项目：先同步调用后端（delete_project → 在线模式转发后端 /api/v1/project/delete，
 * 后端软删除），成功后再移除本地状态；失败抛错由调用方提示。
 */
async function deleteProject(id: string) {
  // 先同步后端，失败抛错，不改动本地
  await repo.remove(id)
  const idx = projects.findIndex(c => c.id === id)
  if (idx >= 0) projects.splice(idx, 1)
  if (selectedProjectId.value === id) clearProject()
}

export function useProjects() {
  return {
    projects,
    selectedProjectId,
    load,
    selectProject,
    clearProject,
    addProject,
    updateProject,
    deleteProject,
    consumeSuppressAutoEnter,
    suppressAutoEnter,
    selectedProjectRole,
  }
}
