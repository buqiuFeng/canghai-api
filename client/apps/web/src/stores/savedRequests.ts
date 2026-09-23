import { ref } from 'vue'
import { defineStore, storeToRefs } from 'pinia'
import '@/stores/pinia'
import type { SavedRequest } from '@/types'

/**
 * 已保存接口列表状态（Phase 8.4：状态收归 Pinia；Phase 7.5：乐观更新原语）。
 *
 * 职责边界：本 store **只持有状态与最小变更原语**（全量替换 / 插入或就地更新 / 删除），
 * 不依赖 repository，也不含在线-离线分支与冲突检测等业务编排 —— 那些仍在
 * `composables/useSavedRequests.ts` 中，从而保证 store 可被单测。
 *
 * 为什么需要 `upsert` / `removeById`：原实现在每次增删改后 `await load()` 全量重拉
 * （一次写操作 = 1 次写 RPC + 1 次全量读 RPC）。改为对本地状态做**局部更新**后，
 * 增删改的读请求归零，列表也不会出现「重拉期间短暂空白」。
 */
export const useSavedRequestsStore = defineStore('savedRequests', () => {
  const savedRequests = ref<SavedRequest[]>([])

  /** 全量替换（仅用于 load 拉取本地库结果） */
  function replaceAll(list: SavedRequest[]) {
    savedRequests.value = list
  }

  /** 插入或就地更新（传入新对象，避免调用方持有同一引用导致变更不被追踪） */
  function upsert(item: SavedRequest) {
    const next = { ...item }
    const idx = savedRequests.value.findIndex(r => r.id === item.id)
    if (idx === -1) savedRequests.value.push(next)
    else savedRequests.value[idx] = next
  }

  /** 按 id 删除 */
  function removeById(id: string) {
    const idx = savedRequests.value.findIndex(r => r.id === id)
    if (idx !== -1) savedRequests.value.splice(idx, 1)
  }

  return { savedRequests, replaceAll, upsert, removeById }
})

/** 兼容导出：既有调用点按 `savedRequests.value` 读写，语义与迁移前的模块级 ref 一致。 */
export const { savedRequests } = storeToRefs(useSavedRequestsStore())
