<script setup lang="ts">
import { watch } from 'vue'
import { useAudit } from '@/composables/useAudit'
import { parseServerTime, formatTime } from '@/utils'

const { logs, loading, query } = useAudit()
const props = defineProps<{ requestId?: string | null }>()
const visible = defineModel<boolean>('visible', { default: false })

const actionLabel: Record<string, string> = {
  create: '创建',
  update: '更新',
  delete: '删除',
  restore: '回退',
}

const entityLabel: Record<string, string> = {
  request: '接口',
  category: '分类',
  environment: '环境',
  environment_group: '环境分组',
  team: '团队',
  member: '成员',
}

watch(visible, (v) => {
  if (v) query({ entityId: props.requestId ?? '', size: 50 })
})
</script>

<template>
  <el-drawer v-model="visible" title="操作审计日志" size="560px" direction="rtl">
    <el-table :data="logs" v-loading="loading" border empty-text="暂无审计记录">
      <el-table-column label="时间" width="150">
        <template #default="{ row }">{{ formatTime(parseServerTime(row.createTime)) }}</template>
      </el-table-column>
      <el-table-column label="操作人" width="90">
        <template #default="{ row }">{{ row.username || '-' }}</template>
      </el-table-column>
      <el-table-column label="操作" width="70">
        <template #default="{ row }">
          <el-tag :type="row.action === 'delete' ? 'danger' : row.action === 'create' ? 'success' : 'info'">
            {{ actionLabel[row.action] || row.action }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column label="对象" min-width="120">
        <template #default="{ row }">
          {{ entityLabel[row.entityType] || row.entityType }} · {{ row.entityName || row.entityId }}
        </template>
      </el-table-column>
    </el-table>
  </el-drawer>
</template>
