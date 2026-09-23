<script setup lang="ts">
import EmptyState from '@/components/EmptyState.vue'
import VersionCompareDialog from '@/components/VersionCompareDialog.vue'
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage } from 'element-plus'
import {
  listRequestVersions,
  getRequestVersionSnapshot,
  restoreRequestVersion,
  type RequestVersionMeta,
  type SavedRequest,
} from '@/composables/useSavedRequests'

const { t } = useI18n()
const props = defineProps<{ requestId: string | null }>()
const emit = defineEmits<{ (e: 'restored', request: SavedRequest): void }>()

/** 参与对比的一侧：版本号 + 该版本完整内容。 */
interface CompareSide {
  version: number
  data: SavedRequest
}

const visible = defineModel<boolean>('visible', { default: false })
const versions = ref<RequestVersionMeta[]>([])
const restoring = ref(false)
/** 勾选用于对比的版本号（最多 2 个） */
const selected = ref<number[]>([])
const comparing = ref(false)
const compareVisible = ref(false)
const leftSide = ref<CompareSide | null>(null)
const rightSide = ref<CompareSide | null>(null)

const canCompare = computed(() => selected.value.length === 2)

async function refresh() {
  if (!props.requestId) return
  const list = await listRequestVersions(props.requestId)
  // 兜底：命令异常返回非数组时不能把 `undefined` 存进列表（`!undefined.length` 会误判为空态）
  versions.value = Array.isArray(list) ? list : []
  selected.value = []
}

watch(
  () => [visible.value, props.requestId],
  ([v]) => {
    if (v && props.requestId) refresh()
  },
)

function toggleSelect(version: number) {
  const idx = selected.value.indexOf(version)
  if (idx >= 0) selected.value.splice(idx, 1)
  else if (selected.value.length < 2) selected.value.push(version)
}

async function onRestore(v: number) {
  if (!props.requestId) return
  restoring.value = true
  try {
    const restored = await restoreRequestVersion(props.requestId, v)
    ElMessage.success(t('version.restore') + ` v${v} ✔`)
    // 回退不产生新版本，刷新列表即可看到「当前版本」标记移动到被回退的版本
    await refresh()
    emit('restored', restored)
  } catch (e: any) {
    ElMessage.error(t('version.restore') + ` ✘: ` + (e?.toString?.() ?? e))
  } finally {
    restoring.value = false
  }
}

async function onCompare() {
  if (!props.requestId || selected.value.length !== 2) return
  comparing.value = true
  try {
    const [a, b] = [...selected.value].sort((x, y) => x - y)
    const [left, right] = await Promise.all([
      getRequestVersionSnapshot(props.requestId, a),
      getRequestVersionSnapshot(props.requestId, b),
    ])
    leftSide.value = { version: a, data: left }
    rightSide.value = { version: b, data: right }
    compareVisible.value = true
  } catch (e: any) {
    ElMessage.error(t('version.compareTitle') + ` ✘: ` + (e?.toString?.() ?? e))
  } finally {
    comparing.value = false
  }
}
</script>

<template>
  <el-drawer v-model="visible" :title="t('version.title')" size="460px" direction="rtl">
    <EmptyState v-if="!versions.length" compact :title="t('version.empty')" />
    <template v-else>
      <div class="vh-hint">{{ t('version.compareHint') }}</div>
      <el-timeline>
        <el-timeline-item
          v-for="item in versions"
          :key="item.id"
          :timestamp="item.create_time"
          placement="top"
        >
          <div class="vh-item">
            <el-checkbox
              :model-value="selected.includes(item.version)"
              :disabled="selected.length >= 2 && !selected.includes(item.version)"
              @change="() => toggleSelect(item.version)"
            />
            <span class="vh-ver">v{{ item.version }}</span>
            <el-tag v-if="item.is_current" size="small" type="success" effect="plain">
              {{ t('version.current') }}
            </el-tag>
            <el-button
              class="vh-restore"
              size="small"
              type="primary"
              plain
              :disabled="item.is_current"
              :loading="restoring"
              @click="onRestore(item.version)"
            >
              {{ t('version.restore') }}
            </el-button>
          </div>
        </el-timeline-item>
      </el-timeline>
    </template>
    <template #footer>
      <el-button :disabled="!canCompare" :loading="comparing" type="primary" @click="onCompare">
        {{ t('version.compareBtn') }}
      </el-button>
    </template>
  </el-drawer>

  <VersionCompareDialog v-model:visible="compareVisible" :left="leftSide" :right="rightSide" />
</template>

<style scoped>
.vh-hint {
  font-size: var(--fs-md);
  color: var(--text-3);
  margin-bottom: 12px;
}
.vh-item {
  display: flex;
  align-items: center;
  gap: 8px;
}
.vh-ver {
  font-weight: bold;
  color: #409eff;
}
.vh-restore {
  margin-left: auto;
}
</style>
