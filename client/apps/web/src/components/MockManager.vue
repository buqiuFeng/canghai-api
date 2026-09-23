<script setup lang="ts">
import EmptyState from '@/components/EmptyState.vue'
import { useI18n } from 'vue-i18n'
import { ElMessage } from 'element-plus'
import { useMock } from '@/composables/useMock'

const { t } = useI18n()
const { routes, port, running, logs, start, stop, addRoute, removeRoute } = useMock()

async function onStart() {
  try {
    await start()
    ElMessage.success(t('mock.start') + ' ✔')
  } catch (e: any) {
    ElMessage.error(t('mock.start') + ' ✘: ' + (e?.toString?.() ?? e))
  }
}
async function onStop() {
  await stop()
  ElMessage.info(t('mock.stop') + ' ✔')
}
</script>

<template>
  <div class="mock-mgr">
    <div class="mock-bar">
      <el-input v-model.number="port" :disabled="running" class="mock-port" type="number">
        <template #prepend>{{ t('mock.port') }}</template>
      </el-input>
      <el-button v-if="!running" type="primary" @click="onStart">{{ t('mock.start') }}</el-button>
      <el-button v-else type="danger" @click="onStop">{{ t('mock.stop') }}</el-button>
      <el-button :disabled="running" @click="addRoute">{{ t('mock.newRoute') }}</el-button>
      <span class="mock-status" :class="running ? 'on' : 'off'">{{ running ? t('mock.running') : t('mock.stopped') }}</span>
    </div>

    <div class="mock-routes">
      <el-table :data="routes" border :empty-text="t('mock.newRoute')">
        <el-table-column :label="t('mock.method')" width="90">
          <template #default="{ row }">
            <el-input v-model="row.method" :disabled="running" size="small" />
          </template>
        </el-table-column>
        <el-table-column :label="t('mock.path')" min-width="140">
          <template #default="{ row }">
            <el-input v-model="row.path" :disabled="running" size="small" placeholder="/api/user 或 /api/user/*" />
          </template>
        </el-table-column>
        <el-table-column :label="t('mock.status')" width="90">
          <template #default="{ row }">
            <el-input v-model.number="row.status" :disabled="running" size="small" />
          </template>
        </el-table-column>
        <el-table-column :label="t('mock.body')" min-width="160">
          <template #default="{ row }">
            <el-input v-model="row.body" :disabled="running" size="small" type="textarea" :rows="1" />
          </template>
        </el-table-column>
        <el-table-column label="SSE" width="70">
          <template #default="{ row }">
            <el-switch v-model="row.sse" :disabled="running" />
          </template>
        </el-table-column>
        <el-table-column label="操作" width="70">
          <template #default="{ $index }">
            <el-button size="small" type="danger" text :disabled="running" @click="removeRoute($index)">删</el-button>
          </template>
        </el-table-column>
      </el-table>
    </div>

    <div class="mock-logs">
      <div class="mock-logs-title">{{ t('ws.connect') }} Log</div>
      <div v-for="(l, i) in logs" :key="i" class="mock-log" :class="l.matched ? 'hit' : 'miss'">
        <span>{{ new Date(l.time_ms).toLocaleTimeString() }}</span>
        <span class="mock-log-method">{{ l.method }}</span>
        <span class="mock-log-path">{{ l.path }}</span>
        <span class="mock-log-status">{{ l.status }}</span>
      </div>
      <EmptyState v-if="!logs.length" compact title="暂无日志" description="启动后访问 mock 地址将记录在此" />
    </div>
  </div>
</template>

<style scoped>
.mock-mgr { padding: 16px; display: flex; flex-direction: column; gap: 14px; height: 100%; box-sizing: border-box; }
.mock-bar { display: flex; gap: 8px; align-items: center; }
.mock-port { width: 160px; }
.mock-status { margin-left: auto; font-size: var(--fs-md); }
.mock-status.on { color: #67c23a; }
.mock-status.off { color: var(--text-3); }
.mock-logs { flex: 1; border: 1px solid var(--el-border-color, #dcdfe6); border-radius: 4px; overflow-y: auto; padding: 8px; }
.mock-logs-title { font-weight: bold; margin-bottom: 6px; color: var(--text-2); }
.mock-log { display: flex; gap: 10px; font-size: var(--fs-sm); font-family: monospace; padding: 2px 0; }
.mock-log.hit .mock-log-status { color: #67c23a; }
.mock-log.miss .mock-log-status { color: #f56c6c; }
.mock-log-empty, .mock-logs-empty { color: var(--text-3); text-align: center; padding: 16px; }
</style>
