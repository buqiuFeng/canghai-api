import { ref, type Ref } from 'vue'
import type { ProjectMemberInfo } from '@/types'
import { useProjectMemberRepo } from '@/repositories/projectMemberRepo'

const repo = useProjectMemberRepo()

export interface ProjectMemberState {
  members: Ref<ProjectMemberInfo[]>
  loading: Ref<boolean>
  load: (projectId: string) => Promise<void>
  addMember: (projectId: string, username: string, role: ProjectMemberInfo['role'], memberType?: 'user' | 'team') => Promise<boolean>
  removeMember: (projectId: string, memberType: 'user' | 'team', memberId: string) => Promise<boolean>
  reset: () => void
}

const members = ref<ProjectMemberInfo[]>([])
const loading = ref(false)

async function load(projectId: string) {
  loading.value = true
  try {
    members.value = await repo.fetchLocal(projectId)
  } finally {
    loading.value = false
  }
}

async function addMember(projectId: string, username: string, role: ProjectMemberInfo['role'], memberType: 'user' | 'team' = 'user') {
  if (!username.trim()) {
    throw new Error('名称不能为空')
  }
  // team 类型成员权限继承自团队本身，不单独配置角色
  const memberRole = memberType === 'team' ? 'inherit' : role
  await repo.add(projectId, memberType, username.trim(), memberRole)
  await load(projectId)
  return true
}

async function removeMember(projectId: string, memberType: 'user' | 'team', memberId: string) {
  await repo.remove(projectId, memberType, memberId)
  members.value = members.value.filter(m => !(m.memberType === memberType && m.memberId === memberId))
  return true
}

function reset() {
  members.value = []
  loading.value = false
}

export function useProjectMembers(): ProjectMemberState {
  return {
    members,
    loading,
    load,
    addMember,
    removeMember,
    reset,
  }
}
