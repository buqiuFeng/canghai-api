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
  </div>
</template>

<script setup lang="ts">
import type { KV } from '@/types'

const props = defineProps<{
  modelValue: KV[]
  placeholderKey?: string
  placeholderValue?: string
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', v: KV[]): void
}>()

const emptyKV = (): KV => ({ key: '', value: '', enabled: true })

function updateRow(idx: number, patch: Partial<KV>) {
  const next = props.modelValue.map((it, i) => (i === idx ? { ...it, ...patch } : it))
  if (idx === next.length - 1 && (patch.key || patch.value)) {
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
.kv-key { flex: 1; }
.kv-key :deep(input) {
  font-family: 'JetBrains Mono', ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: var(--fs-sm);
  letter-spacing: -0.01em;
}
.kv-val { flex: 2; }
.kv-val :deep(input) {
  font-size: var(--fs-sm);
}
</style>
