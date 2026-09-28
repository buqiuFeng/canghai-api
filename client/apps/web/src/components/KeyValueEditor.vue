<template>
  <div class="kv-editor">
    <div v-for="(row, idx) in modelValue" :key="idx" class="kv-row">
      <el-checkbox
        :model-value="row.enabled"
        @update:model-value="(v: boolean | string | number) => updateRow(idx, { enabled: Boolean(v) })"
      />
      <el-input
        :model-value="row.key"
        :placeholder="placeholderKey || 'key'"
        class="kv-key"
        @update:model-value="(v: string) => updateRow(idx, { key: v })"
      />
      <el-input
        :model-value="row.value"
        :placeholder="placeholderValue || 'value'"
        class="kv-val"
        @update:model-value="(v: string) => updateRow(idx, { value: v })"
      />
      <el-input
        :model-value="row.description"
        placeholder="描述"
        class="kv-desc"
        @update:model-value="(v: string) => updateRow(idx, { description: v })"
      />
      <el-button
        link
        type="danger"
        size="small"
        :disabled="modelValue.length === 1 && !row.key && !row.value"
        @click="removeRow(idx)"
      >
        删除
      </el-button>
    </div>
    <!-- 显式新增入口：导入的空数组（0 行）也能添加字段 -->
    <div class="kv-add-row">
      <el-button link type="primary" size="small" :icon="Plus" @click="addRow">
        {{ addText || '添加字段' }}
      </el-button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { Plus } from '@element-plus/icons-vue'
import { emptyKV, type KV } from '@/types'

const props = defineProps<{
  modelValue: KV[]
  placeholderKey?: string
  placeholderValue?: string
  /** 「添加」按钮文案，不同场景可定制（参数/Header/字段） */
  addText?: string
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', v: KV[]): void
}>()

/** 追加一行空记录。用于「导入后为空数组（0 行）」或需显式新增的场景。 */
function addRow() {
  emit('update:modelValue', [...props.modelValue, emptyKV()])
}

function updateRow(idx: number, patch: Partial<KV>) {
  const next = props.modelValue.map((it, i) => (i === idx ? { ...it, ...patch } : it))
  if (idx === next.length - 1 && (patch.key || patch.value || patch.description)) {
    next.push(emptyKV())
  }
  emit('update:modelValue', next)
}

function removeRow(idx: number) {
  const next = props.modelValue.filter((_, i) => i !== idx)
  if (!next.length) next.push(emptyKV())
  emit('update:modelValue', next)
}
</script>

<style scoped>
.kv-editor {
  display: flex;
  flex-direction: column;
  gap: 3px;
}
.kv-add-row {
  padding: 2px 6px;
}
.kv-row {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 6px;
  border-radius: 6px;
  transition: background 0.12s cubic-bezier(0.16, 1, 0.3, 1);
}
.kv-row:hover {
  background: rgba(99,102,241,0.03);
}
.kv-key { flex: 1 1 0; min-width: 0; }
.kv-key :deep(input) {
  font-family: 'JetBrains Mono', ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: var(--fs-sm);
  letter-spacing: -0.01em;
}
.kv-val { flex: 2 1 0; min-width: 0; }
.kv-val :deep(input) {
  font-size: var(--fs-sm);
}
.kv-desc { flex: 2 1 0; min-width: 0; }
.kv-desc :deep(input) {
  font-size: var(--fs-sm);
}
/* 删除按钮始终可见，不参与收缩 */
.kv-row > .el-button {
  flex: 0 0 auto;
}
</style>
