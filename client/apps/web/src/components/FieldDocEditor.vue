<template>
  <div class="field-doc-editor">
    <div v-for="(row, idx) in modelValue" :key="idx" class="fde-row">
      <el-input
        :model-value="row.key"
        placeholder="字段名（支持 data.list[].id 路径）"
        class="fde-key"
        @update:model-value="(v: string) => updateRow(idx, { key: v })"
      />
      <el-select
        :model-value="row.fieldType"
        size="small"
        class="fde-type"
        @update:model-value="(v: string) => updateRow(idx, { fieldType: v })"
      >
        <el-option v-for="opt in FIELD_TYPE_OPTIONS" :key="opt" :label="opt" :value="opt" />
      </el-select>
      <el-checkbox
        :model-value="row.required"
        class="fde-required"
        @update:model-value="(v: boolean | string | number) => updateRow(idx, { required: Boolean(v) })"
      >必填</el-checkbox>
      <el-input
        :model-value="row.description"
        placeholder="字段描述"
        class="fde-desc"
        @update:model-value="(v: string) => updateRow(idx, { description: v })"
      />
      <el-button
        link
        type="danger"
        size="small"
        :disabled="isOnlyEmptyRow(idx)"
        @click="removeRow(idx)"
      >删除</el-button>
    </div>
    <!-- 显式新增入口：导入的空数组（0 行）也能添加字段；同时提供操作插槽放「生成」按钮 -->
    <div class="fde-add-row">
      <el-button link type="primary" size="small" :icon="Plus" @click="addRow">
        {{ addText || '添加字段' }}
      </el-button>
      <slot name="actions" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { Plus } from '@element-plus/icons-vue'
import { FIELD_TYPE_OPTIONS, emptyFieldDoc, type FieldDoc } from '@/types'

const props = defineProps<{
  modelValue: FieldDoc[]
  /** 「添加」按钮文案，不同场景可定制 */
  addText?: string
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', v: FieldDoc[]): void
}>()

/** 追加一行空记录。用于「导入后为空数组（0 行）」或需显式新增的场景。 */
function addRow() {
  emit('update:modelValue', [...props.modelValue, emptyFieldDoc()])
}

function updateRow(idx: number, patch: Partial<FieldDoc>) {
  const next = props.modelValue.map((it, i) => (i === idx ? { ...it, ...patch } : it))
  if (idx === next.length - 1 && (patch.key || patch.description)) {
    next.push(emptyFieldDoc())
  }
  emit('update:modelValue', next)
}

function removeRow(idx: number) {
  const next = props.modelValue.filter((_, i) => i !== idx)
  if (!next.length) next.push(emptyFieldDoc())
  emit('update:modelValue', next)
}

/** 仅剩一行且该行为空时禁用删除（与 KeyValueEditor 行为一致） */
function isOnlyEmptyRow(idx: number): boolean {
  const row = props.modelValue[idx]
  return props.modelValue.length === 1 && !row.key.trim() && !row.description.trim()
}
</script>

<style scoped>
.field-doc-editor {
  display: flex;
  flex-direction: column;
  gap: 3px;
}
.fde-add-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 2px 6px;
}
.fde-row {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 6px;
  border-radius: 6px;
  transition: background 0.12s cubic-bezier(0.16, 1, 0.3, 1);
}
.fde-row:hover {
  background: rgba(99,102,241,0.03);
}
.fde-key { flex: 2 1 0; min-width: 0; }
.fde-key :deep(input) {
  font-family: 'JetBrains Mono', ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: var(--fs-sm);
  letter-spacing: -0.01em;
}
.fde-type {
  flex: 0 0 96px;
  width: 96px;
}
.fde-required {
  flex: 0 0 auto;
  white-space: nowrap;
  margin-right: 2px;
}
.fde-required :deep(.el-checkbox__label) {
  padding-left: 4px;
  font-size: var(--fs-sm);
}
.fde-desc { flex: 3 1 0; min-width: 0; }
.fde-desc :deep(input) {
  font-size: var(--fs-sm);
}
/* 删除按钮始终可见，不参与收缩 */
.fde-row > .el-button {
  flex: 0 0 auto;
}
</style>
