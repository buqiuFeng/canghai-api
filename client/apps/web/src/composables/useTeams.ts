import { ref, computed } from 'vue'
import { currentUser, currentTeamId, isLoggedIn } from './useSession'
import { useDataMode } from './useDataMode'
import { useTeamRepo } from '@/repositories/teamRepo'
// 共享类型统一收敛到 @/types（Phase 8.3：打断 repository ↔ composable 循环），
// 此处 re-export 兼容既有 import 路径（useTeams 仍作为对外类型入口）。
import type { MemberInfo, MemberRole, Team } from '@/types'

export type { MemberInfo, MemberRole, Team }

const repo = useTeamRepo()

export const MEMBER_ROLES: { value: MemberRole; label: string; desc: string }[] = [
  { value: 'admin',     label: '管理员',   desc: '可管理成员和编辑团队' },
  { value: 'readwrite', label: '读写',     desc: '可同步上传和拉取数据' },
  { value: 'readonly',  label: '只读',     desc: '仅可查看和拉取数据' },
]

export const ROLE_LABEL_MAP: Record<MemberRole, string> = {
  owner:     '所有者',
  admin:     '管理员',
  readwrite: '读写',
  readonly:  '只读',
}

export const ROLE_TAG_TYPE: Record<MemberRole, string> = {
  owner:     'danger',
  admin:     'warning',
  readwrite: 'success',
  readonly:  'info',
}

/** 团队列表 */
export const teams = ref<Team[]>([])

/** 当前团队成员 */
export const teamMembers = ref<MemberInfo[]>([])

/** 当前用户在当前团队中的角色（成员列表为空时返回 null 表示未知） */
export const currentUserRole = computed<MemberRole | null>(() => {
  const userId = currentUser.value?.id
  if (!userId) return null
  const m = teamMembers.value.find(m => m.userId === userId)
  return m?.role ?? null
})

/** 成员数据是否已成功加载完成（false 表示尚未加载或加载失败）。
 *  仅当「已加载且列表为空」（如离线默认团队）时才放行写入；
 *  未加载完成时保持保守的只读策略，避免绕过权限编辑团队数据（对应清单 M7）。 */
const membersLoaded = ref(false)

/** 当前是否处于离线默认团队（离线模式下允许完整本地操作）。 */
const isOfflineTeam = computed(() => {
  const id = (teams.value.find(t => t.id)?.id) ?? currentTeamIdFallback()
  return id === 'default'
})
function currentTeamIdFallback(): string {
  return 'default'
}

/** 当前用户是否可写（owner / admin / readwrite）。
 *  成员未加载完成时降级为只读；离线默认团队或已确认成员为空时放行。 */
export const canWrite = computed(() => {
  if (!membersLoaded.value && !isOfflineTeam.value) return false
  const role = currentUserRole.value
  return role === null
    ? (isOfflineTeam.value)
    : (role === 'owner' || role === 'admin' || role === 'readwrite')
})

/** 当前用户是否是管理员或以上（owner / admin）。
 *  成员未加载完成时降级为只读。 */
export const isAdminOrAbove = computed(() => {
  if (!membersLoaded.value && !isOfflineTeam.value) return false
  const role = currentUserRole.value
  return role === null
    ? (isOfflineTeam.value)
    : (role === 'owner' || role === 'admin')
})

/** 当前用户是否是所有者。
 *  成员未加载完成时降级为只读。 */
export const isOwner = computed(() => {
  if (!membersLoaded.value && !isOfflineTeam.value) return false
  const role = currentUserRole.value
  return role === 'owner'
})

/** 加载标志 */
export const teamsLoading = ref(false)

/** 重新加载团队列表 */
export async function loadTeams(): Promise<void> {
  // 离线模式下云端不可达：不再调用 get_teams，保留本地已有的团队（或离线默认团队）。
  if (useDataMode().dataMode.value === 'offline') {
    teamsLoading.value = false
    return
  }
  // 在线模式但未登录：Rust 侧无 token 会直接返回 401（不会真正打到后端），
  // 此处提前跳过，避免未登录进入项目页 / 调试页时产生无意义的 get_teams 调用。
  // 兜底行为与 401 分支保持一致：列表为空时补一个占位团队，避免顶栏/选择器空态。
  if (!isLoggedIn.value) {
    if (teams.value.length === 0) {
      teams.value = [
        { id: 'default', name: '离线团队', description: '' },
      ]
    }
    teamsLoading.value = false
    return
  }
  teamsLoading.value = true
  try {
    const list = await repo.fetchTeams()
    if (list) {
      teams.value = list
    }
  } catch {
    if (teams.value.length === 0) {
      teams.value = [
        { id: 'default', name: '离线团队', description: '' },
      ]
    }
  } finally {
    teamsLoading.value = false
  }
}

/** 创建团队 */
export async function createTeam(name: string, description: string): Promise<Team> {
  const t = await repo.create(name, description)
  await loadTeams()
  return t
}

/** 更新团队 */
export async function updateTeam(id: string, name: string, description: string): Promise<void> {
  await repo.update(id, name, description)
  await loadTeams()
}

/** 删除团队 */
export async function deleteTeam(id: string): Promise<void> {
  await repo.remove(id)
  await loadTeams()
}

/** 邀请成员 */
export async function inviteMember(teamId: string, username: string, role: MemberRole = 'readwrite'): Promise<string> {
  const msg = await repo.invite(teamId, username, role)
  await loadTeamMembers(teamId)
  return msg
}

/** 修改成员角色 */
export async function changeMemberRole(teamId: string, userId: string, role: MemberRole): Promise<string> {
  const msg = await repo.changeRole(teamId, userId, role)
  await loadTeamMembers(teamId)
  return msg
}

/** 移除成员 */
export async function removeMember(teamId: string, userId: string): Promise<string> {
  const msg = await repo.removeMember(teamId, userId)
  await loadTeamMembers(teamId)
  return msg
}

/** 移交团队 */
export async function transferTeam(teamId: string, newOwnerUsername: string): Promise<string> {
  const msg = await repo.transfer(teamId, newOwnerUsername)
  await loadTeamMembers(teamId)
  await loadTeams()
  return msg
}

/** 加载成员列表 */
export async function loadTeamMembers(teamId: string): Promise<void> {
  // 离线模式 / 在线但未登录：云端不可达（未登录时 get_team_members 必然 401），
  // 不发起调用，保持未加载态（降级只读）。
  if (useDataMode().dataMode.value === 'offline' || !isLoggedIn.value) {
    membersLoaded.value = false
    return
  }
  try {
    const list = await repo.fetchMembers(teamId)
    if (list) {
      teamMembers.value = list
    }
    // 无论是否成功拿到数据，视为已尝试加载（完成）
    membersLoaded.value = true
  } catch {
    teamMembers.value = []
    // 加载失败：保持未加载态（降级只读），避免以空列表误放行写权限
    membersLoaded.value = false
  }
}

/**
 * 加载「当前用户所属团队」(currentTeamId) 的成员，用于计算 canWrite / isAdminOrAbove 等权限。
 * canWrite 依赖 teamMembers 与 membersLoaded，若未加载则降级为只读，
 * 表现为接口页头部的环境选择器、保存按钮等被禁用（不可点击）。
 *
 * 该加载原本仅在 TeamManager 组件挂载时触发，刷新页面后若未进入团队管理页，
 * 权限状态无法恢复。故在此集中提供，供登录后 / 进入需要写权限的页面前统一调用。
 */
export async function loadCurrentTeamMembers(): Promise<void> {
  const teamId = currentTeamId.value
  if (!teamId) return
  await loadTeamMembers(teamId)
}

export function useTeams() {
  return {
    teams,
    teamMembers,
    currentUserRole,
    canWrite,
    isAdminOrAbove,
    isOwner,
    teamsLoading,
    loadTeams,
    createTeam,
    updateTeam,
    deleteTeam,
    inviteMember,
    changeMemberRole,
    removeMember,
    transferTeam,
    loadTeamMembers,
    loadCurrentTeamMembers,
  }
}
