import { invokeUnwrap } from '@/lib/tauri'
import type { Team, MemberInfo, MemberRole } from '@/types'

/**
 * 团队数据仓库 —— 收敛团队 / 团队成员的后端命令（均经 Rust 转发后端，离线模式上层不发请求）。
 *
 * 说明：Rust `team.rs` 的命令统一返回 `ApiResult<T>` 信封，故此处必须经 `invokeUnwrap` 拆包；
 * 此前直接 `invoke<Team>` 会把整个信封当业务数据返回（失败也被当成功），属契约漂移缺陷。
 */
export function useTeamRepo() {
  return {
    /** 读取团队列表；失败返回 null（由调用方决定是否保留现有列表）。 */
    async fetchTeams(): Promise<Team[] | null> {
      try {
        return (await invokeUnwrap<Team[]>('get_teams')) ?? []
      } catch (e) {
        console.warn('[teamRepo] 读取团队列表失败', e)
        return null
      }
    },
    create(name: string, description: string) {
      return invokeUnwrap<Team>('create_team', { name, description })
    },
    update(id: string, name: string, description: string) {
      return invokeUnwrap<void>('update_team', { id, name, description })
    },
    remove(id: string) {
      return invokeUnwrap<void>('delete_team', { id })
    },
    invite(teamId: string, username: string, role: MemberRole) {
      return invokeUnwrap<string>('invite_member', { teamId, username, role })
    },
    changeRole(teamId: string, userId: string, role: MemberRole) {
      return invokeUnwrap<string>('change_member_role', { teamId, userId, role })
    },
    removeMember(teamId: string, userId: string) {
      return invokeUnwrap<string>('remove_member', { teamId, userId })
    },
    transfer(teamId: string, newOwnerUsername: string) {
      return invokeUnwrap<string>('transfer_team', { teamId, newOwnerUsername })
    },
    /** 读取团队成员；失败返回 null。 */
    async fetchMembers(teamId: string): Promise<MemberInfo[] | null> {
      try {
        return (await invokeUnwrap<MemberInfo[]>('get_team_members', { teamId })) ?? []
      } catch (e) {
        console.warn('[teamRepo] 读取团队成员失败', e)
        return null
      }
    },
  }
}
