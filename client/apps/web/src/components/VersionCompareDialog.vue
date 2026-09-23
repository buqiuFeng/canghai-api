<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { SavedRequest } from '@/types'

/** 参与对比的一侧：版本号 + 该版本完整内容。 */
interface CompareSide {
  version: number
  data: SavedRequest
}

const { t } = useI18n()
const props = defineProps<{ left: CompareSide | null; right: CompareSide | null }>()
const visible = defineModel<boolean>('visible', { default: false })

interface FieldDef {
  key: keyof SavedRequest
  label: string
}

const fields = computed<FieldDef[]>(() => [
  { key: 'name', label: t('version.fields.name') },
  { key: 'method', label: t('version.fields.method') },
  { key: 'url', label: t('version.fields.url') },
  { key: 'params', label: t('version.fields.params') },
  { key: 'headers', label: t('version.fields.headers') },
  { key: 'bodyType', label: t('version.fields.bodyType') },
  { key: 'body', label: t('version.fields.body') },
  { key: 'formBody', label: t('version.fields.formBody') },
  { key: 'categoryId', label: t('version.fields.categoryId') },
  { key: 'preScript', label: t('version.fields.preScript') },
  { key: 'postScript', label: t('version.fields.postScript') },
])

/** 取字段展示值：对象统一格式化为 JSON，便于肉眼比对。 */
function val(side: CompareSide | null, key: keyof SavedRequest): string {
  const v = side?.data?.[key]
  if (v === null || v === undefined) return ''
  if (typeof v === 'object') return JSON.stringify(v, null, 2)
  return String(v)
}

function isDiff(key: keyof SavedRequest): boolean {
  return val(props.left, key) !== val(props.right, key)
}

const diffCount = computed(() => fields.value.filter(f => isDiff(f.key)).length)
</script>

<template>
  <el-dialog v-model="visible" :title="t('version.compareTitle')" width="880px" top="8vh">
    <div class="vc-tip">
      {{ t('version.diffSummary', { n: diffCount }) }}
    </div>
    <div class="vc-scroll">
      <table class="vc-table">
        <thead>
          <tr>
            <th class="vc-col-field">{{ t('version.fieldHeader') }}</th>
            <th class="vc-col-side">v{{ left?.version ?? '-' }}</th>
            <th class="vc-col-side">v{{ right?.version ?? '-' }}</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="f in fields" :key="f.key" :class="{ 'vc-row-diff': isDiff(f.key) }">
            <td class="vc-col-field">{{ f.label }}</td>
            <td class="vc-col-side">{{ val(left, f.key) }}</td>
            <td class="vc-col-side">{{ val(right, f.key) }}</td>
          </tr>
        </tbody>
      </table>
    </div>
    <template #footer>
      <el-button @click="visible = false">{{ t('common.ok') }}</el-button>
    </template>
  </el-dialog>
</template>

<style scoped>
.vc-tip {
  margin-bottom: 10px;
  font-size: var(--fs-md);
  color: var(--text-3);
}
.vc-scroll {
  max-height: 62vh;
  overflow: auto;
}
.vc-table {
  width: 100%;
  border-collapse: collapse;
  font-size: var(--fs-md);
  table-layout: fixed;
}
.vc-table th,
.vc-table td {
  border: 1px solid var(--border-color, #eee);
  padding: 8px 10px;
  text-align: left;
  vertical-align: top;
  word-break: break-all;
  white-space: pre-wrap;
}
.vc-table th {
  position: sticky;
  top: 0;
  background: var(--surface-2);
  font-weight: 600;
  z-index: 1;
}
.vc-col-field {
  width: 120px;
  color: #888;
}
.vc-col-side {
  width: calc((100% - 120px) / 2);
}
.vc-row-diff {
  background: var(--tint-warning);
}
.vc-row-diff .vc-col-side {
  font-weight: 600;
}
</style>
