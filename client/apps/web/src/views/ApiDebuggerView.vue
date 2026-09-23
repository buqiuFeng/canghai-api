<template>
  <AppLayout>
    <template #header>
      <div class="topbar-project" v-if="projectId">
        <el-tag type="primary" effect="plain" class="proj-tag">
          <el-icon><FolderOpened /></el-icon>
          {{ projectName }}
        </el-tag>
        <el-button size="small" text :icon="Switch" title="切换项目" @click="switchProject">
          切换
        </el-button>
      </div>
      <div class="topbar-right">
        <!-- 组1：上下文（环境 + 工作区） -->
        <div class="topbar-group">
          <div class="env-selector">
            <el-icon><Setting /></el-icon>
            <el-select
              :model-value="activeEnv?.id ?? ''"
              placeholder="无环境"
              size="small"
              class="env-select"
              :disabled="!canWrite"
              :title="canWrite ? '' : '只读成员不能切换环境'"
              @change="handleEnvSwitch"
            >
              <el-option-group
                v-for="g in groupedEnvs.grouped"
                :key="g.group.id"
                :label="g.group.name"
              >
                <el-option
                  v-for="env in g.envs"
                  :key="env.id"
                  :label="env.name"
                  :value="env.id"
                >
                  <span class="env-option">
                    <span>{{ env.name }}</span>
                    <el-tag v-if="env.isActive" size="small" type="success" effect="plain">当前</el-tag>
                  </span>
                </el-option>
              </el-option-group>
              <el-option
                v-for="env in groupedEnvs.ungrouped"
                :key="env.id"
                :label="env.name"
                :value="env.id"
              >
                <span class="env-option">
                  <span>{{ env.name }}</span>
                  <el-tag v-if="env.isActive" size="small" type="success" effect="plain">当前</el-tag>
                </span>
              </el-option>
            </el-select>
            <el-button size="small" link :disabled="!canWrite" :title="canWrite ? '环境管理' : '只读成员不能管理环境'" @click="openEnvManager">
              <el-icon><EditPen /></el-icon>
            </el-button>
          </div>

        </div>
        <span class="topbar-sep"></span>
        <!-- 组2：工具（历史 + 设置） -->
        <div class="topbar-group">
          <el-button
            size="small"
            :type="showHistory ? 'primary' : 'default'"
            text
            class="btn-toggle-history"
            @click="toggleHistory"
            :title="showHistory ? '隐藏历史记录' : '显示历史记录'"
          >
            <el-icon><Clock /></el-icon>
            历史
          </el-button>
        </div>
      </div>
      <EnvironmentManager ref="envManagerRef" />
    </template>

    <main class="content" :style="contentGridStyle">
      <!-- 左：分类树 -->
      <aside class="card sidebar-card" ref="sidebarRef">
        <CategoryTree
          :project-id="projectId"
          :selected-request-id="currentRequestId"
          @select-request="loadSavedRequest"
          @delete-request="handleDeleteSavedRequest"
          @save-new-request="handleSaveNewRequest"
        />
        <div
          class="sidebar-resize-handle"
          @mousedown="startSidebarResize"
        />
      </aside>

      <!-- 中：请求 + 响应 -->
      <section class="card main-card">
        <!-- 首次进入：系统说明页 -->
        <template v-if="showWelcome">
          <div class="welcome-panel">
            <div class="welcome-hero">
              <div class="welcome-logo">
                <el-icon :size="26"><component :is="Cloudy" /></el-icon>
              </div>
              <h1 class="welcome-title">沧海·API 调试工具</h1>
              <p class="welcome-sub">轻量级跨平台 HTTP 调试桌面应用 · Tauri 原生代理，无 CORS 限制</p>
            </div>

            <div class="welcome-features">
              <div class="welcome-feature" v-for="f in welcomeFeatures" :key="f.title">
                <div class="wf-icon"><el-icon :size="18"><component :is="f.icon" /></el-icon></div>
                <div class="wf-body">
                  <div class="wf-title">{{ f.title }}</div>
                  <div class="wf-desc">{{ f.desc }}</div>
                </div>
              </div>
            </div>

            <div class="welcome-steps">
              <div class="ws-step" v-for="(s, i) in welcomeSteps" :key="i">
                <div class="ws-num">{{ i + 1 }}</div>
                <div>
                  <div class="ws-title">{{ s.title }}</div>
                  <div class="ws-desc">{{ s.desc }}</div>
                </div>
              </div>
            </div>

            <div class="welcome-footer">
              <el-button type="primary" size="large" :icon="Position" class="welcome-cta" @click="startDebugging">
                开始调试
              </el-button>
              <div class="welcome-tip">也可以从左侧「接口分类」或「历史记录」中打开已有请求</div>
            </div>
          </div>
        </template>

        <template v-else-if="tabs.length">
        <!-- 页签栏 -->
        <div class="tab-bar">
          <div class="tab-list" ref="tabListRef">
            <div
              v-for="tab in tabs"
              :key="tab.id"
              class="tab-item"
              :class="{ active: tab.id === activeTabId }"
              @click="switchTab(tab.id)"
              @contextmenu.prevent="openTabContextMenu($event, tab.id)"
            >
              <span class="tab-method" :class="'tm-' + tab.form.method.toLowerCase()">{{ tab.form.method.slice(0, 3) }}</span>
              <template v-if="editingTabId === tab.id">
                <input
                  ref="tabInputRef"
                  class="tab-title-input"
                  v-model="editingTabTitle"
                  @blur="finishEditTabTitle"
                  @keyup.enter="finishEditTabTitle"
                  @keyup.escape="cancelEditTabTitle"
                  @click.stop
                />
              </template>
              <template v-else>
                <span
                  class="tab-title"
                  :class="{ 'tab-title-empty': !tab.title }"
                  :title="tab.title || '新请求'"
                  @dblclick.stop="startEditTabTitle(tab.id)"
                >{{ tab.title || '新请求' }}</span>
              </template>
              <span
                v-if="tabs.length > 1"
                class="tab-close"
                @click.stop="closeTab(tab.id)"
                title="关闭"
              >&times;</span>
            </div>
          </div>
          <button class="tab-add-btn" @click="addTab" title="新建请求">+</button>
        </div>

        <!-- 页签右键菜单 -->
        <Teleport to="body">
          <div
            v-if="tabContextMenu.visible"
            class="tab-context-menu"
            :style="{ left: tabContextMenu.x + 'px', top: tabContextMenu.y + 'px' }"
            @click="tabContextMenu.visible = false"
            @contextmenu.prevent
          >
            <div class="tab-ctx-item" @click="ctxDuplicateTab">复制页签</div>
            <div class="tab-ctx-divider" />
            <div class="tab-ctx-item" @click="ctxCloseTab">关闭</div>
            <div class="tab-ctx-item" :class="{ disabled: tabs.length <= 1 }" @click="ctxCloseOthers">关闭其他</div>
            <div class="tab-ctx-item" :class="{ disabled: tabs.length <= 1 }" @click="ctxCloseAll">关闭所有</div>
            <div class="tab-ctx-item" :class="{ disabled: !hasTabsToRight }" @click="ctxCloseToRight">关闭右侧</div>
          </div>
        </Teleport>

        <!-- 工作模式切换器 + 标题 -->
        <div class="mode-switcher">
          <div class="mode-btns">
            <button
              v-for="m in workModes"
              :key="m.key"
              class="mode-btn"
              :class="{ active: workMode === m.key }"
              @click="workMode = m.key"
            >
              <el-icon :size="14"><component :is="m.icon" /></el-icon>
              <span>{{ m.label }}</span>
            </button>
          </div>
          <el-input
            v-model="currentTabTitle"
            placeholder="输入接口名称"
            class="title-input"
            variant="borderless"
          />
        </div>

        <!-- ====== 调试模式 ====== -->
        <div v-if="workMode === 'debug'" class="debug-panel">

        <!-- URL 行 -->
        <div class="url-row">
          <el-select v-model="form.method" class="method-select" :class="'method-' + form.method.toLowerCase()">
            <el-option v-for="m in METHODS" :key="m" :label="m" :value="m" />
          </el-select>
          <el-input
            v-model="form.url"
            placeholder="https://api.example.com/path"
            class="url-input"
            clearable
            @keyup.enter="sendRequest"
          />
          <el-popover :width="240" trigger="click" v-model:visible="urlVarVisible">
            <template #reference>
              <el-button :icon="Coin" class="btn-var" title="插入环境变量（先聚焦 URL 再点此）" @mousedown.prevent="captureFocusedInput" />
            </template>
            <div class="var-pop">
              <div v-if="!activeVariables.length" class="var-pop-empty">当前环境暂无变量</div>
              <div
                v-for="v in activeVariables"
                v-else
                :key="v.key"
                class="var-pop-item"
                :class="{ off: !v.enabled }"
                @click="insertVar(v.key); urlVarVisible = false"
              >
                <span class="vp-k">&#123;&#123;{{ v.key }}&#125;&#125;</span>
                <span class="vp-v">{{ v.enabled ? (v.value || '（空）') : '（已禁用）' }}</span>
              </div>
            </div>
          </el-popover>
          <el-button
            type="primary"
            :loading="loading"
            :icon="Position"
            class="btn-send"
            @click="sendRequest"
          >
            {{ t('app.send') }}
          </el-button>
          <el-button
            :icon="Star"
            :type="currentRequestId ? 'warning' : 'default'"
            :class="{ 'btn-save-update': !!currentRequestId }"
            :disabled="!canWrite"
            @click="saveCurrentRequest"
            :title="canWrite ? (currentRequestId ? '覆盖保存' : '新建保存') + ' (Ctrl+S)' : '只读成员不能保存'"
          >
            {{ currentRequestId ? t('app.update') : t('app.save') }}
          </el-button>
          <el-button
            :icon="Clock"
            :disabled="!currentRequestId"
            @click="openVersionHistory"
            title="查看版本历史并回退"
          >
            {{ t('app.version') }}
          </el-button>
          <el-button
            :icon="Cloudy"
            @click="mockDrawerVisible = true"
            title="本地 Mock 服务"
          >
            {{ t('app.mock') }}
          </el-button>
          <el-tooltip :content="t('app.sseTooltip')" placement="bottom">
            <el-checkbox v-model="streamMode" size="small" class="sse-switch">
              {{ t('app.sseStream') }}
            </el-checkbox>
          </el-tooltip>
        </div>

        <!-- 分类选择 + 正在编辑面包屑 -->
        <div class="meta-row">
          <!-- 正在编辑面包屑 -->
          <div v-if="currentRequestId && currentRequestName" class="editing-breadcrumb">
            <el-icon :size="12" class="editing-icon"><Document /></el-icon>
            <span class="editing-label">编辑中</span>
            <template v-if="editingBreadcrumb.length">
              <span class="bc-sep">/</span>
              <span v-for="(bc, i) in editingBreadcrumb" :key="i" class="bc-item">{{ bc }}</span>
            </template>
            <span class="bc-sep">/</span>
            <span class="bc-current">{{ currentRequestName }}</span>
          </div>
          <!-- 分类选择 -->
          <div class="category-picker" :class="{ 'ml-auto': currentRequestId && currentRequestName }">
            <el-icon><Folder /></el-icon>
            <el-tree-select
              v-model="form.categoryId"
              :data="categoryTreeData"
              :props="{ value: 'id', label: 'name', children: 'children' }"
              placeholder="选择接口分类（可选）"
              clearable
              check-strictly
              filterable
              class="cat-select"
              size="small"
            />
          </div>
        </div>

        <!-- 请求参数 Tabs -->
        <el-tabs v-model="reqTab" class="req-tabs">
          <el-tab-pane name="params">
            <template #label>
              <span class="tab-label-with-dot">Params<span v-if="hasParams" class="tab-dot" /></span>
            </template>
            <KeyValueEditor v-model="form.params" placeholder-key="参数名" placeholder-value="参数值" />
          </el-tab-pane>
          <el-tab-pane name="headers">
            <template #label>
              <span class="tab-label-with-dot">Headers<span v-if="hasHeaders" class="tab-dot" /></span>
            </template>
            <KeyValueEditor v-model="form.headers" placeholder-key="Header 名" placeholder-value="Header 值" />
          </el-tab-pane>
          <el-tab-pane name="body" :disabled="!methodAllowsBody">
            <template #label>
              <span class="tab-label-with-dot">Body<span v-if="hasBody" class="tab-dot" /></span>
            </template>
            <div class="body-bar">
              <el-radio-group v-model="form.bodyType" size="small">
                <el-radio-button value="none">none</el-radio-button>
                <el-radio-button value="json">JSON</el-radio-button>
                <el-radio-button value="form">form-urlencoded</el-radio-button>
                <el-radio-button value="text">text</el-radio-button>
              </el-radio-group>
              <el-button
                v-if="form.bodyType === 'json'"
                size="small"
                link
                type="primary"
                @click="formatJsonBody"
              >格式化 JSON</el-button>
              <el-popover :width="240" trigger="click" v-model:visible="bodyVarVisible">
                <template #reference>
                  <el-button :icon="Coin" size="small" class="btn-var" title="插入环境变量（先聚焦请求体再点此）" @mousedown.prevent="captureFocusedInput" />
                </template>
                <div class="var-pop">
                  <div v-if="!activeVariables.length" class="var-pop-empty">当前环境暂无变量</div>
                  <div
                    v-for="v in activeVariables"
                    v-else
                    :key="v.key"
                    class="var-pop-item"
                    :class="{ off: !v.enabled }"
                    @click="insertVar(v.key); bodyVarVisible = false"
                  >
                    <span class="vp-k">&#123;&#123;{{ v.key }}&#125;&#125;</span>
                    <span class="vp-v">{{ v.enabled ? (v.value || '（空）') : '（已禁用）' }}</span>
                  </div>
                </div>
              </el-popover>
            </div>
            <KeyValueEditor
              v-if="form.bodyType === 'form'"
              v-model="form.formBody"
              placeholder-key="字段名"
              placeholder-value="字段值"
            />
            <el-input
              v-else-if="form.bodyType !== 'none'"
              v-model="form.body"
              type="textarea"
              :rows="10"
              :placeholder="form.bodyType === 'json' ? '{\n  // 支持注释\n  &quot;key&quot;: &quot;value&quot;\n}' : '请求体文本'"
              class="body-textarea"
            />
            <EmptyState v-else compact title="无请求体" description="该方法或当前 Body 类型无请求体" />
          </el-tab-pane>
          <el-tab-pane name="preScript">
            <template #label>
              <span class="tab-label-with-dot">前置脚本<span v-if="hasPreScript" class="tab-dot" /></span>
            </template>
            <div class="script-bar">
              <span class="script-hint">在发送请求前执行 · 支持 async/await</span>
              <el-button
                v-if="!form.preScript.trim()"
                size="small" link type="primary"
                @click="form.preScript = PRE_SCRIPT_TEMPLATE"
              >插入模板</el-button>
              <el-button
                v-else
                size="small" link type="danger"
                @click="form.preScript = ''"
              >清空</el-button>
              <el-popover :width="240" trigger="click" v-model:visible="preVarVisible">
                <template #reference>
                  <el-button :icon="Coin" size="small" class="btn-var" title="插入环境变量（先聚焦脚本再点此）" @mousedown.prevent="captureFocusedInput" />
                </template>
                <div class="var-pop">
                  <div v-if="!activeVariables.length" class="var-pop-empty">当前环境暂无变量</div>
                  <div
                    v-for="v in activeVariables"
                    v-else
                    :key="v.key"
                    class="var-pop-item"
                    :class="{ off: !v.enabled }"
                    @click="insertVar(v.key); preVarVisible = false"
                  >
                    <span class="vp-k">&#123;&#123;{{ v.key }}&#125;&#125;</span>
                    <span class="vp-v">{{ v.enabled ? (v.value || '（空）') : '（已禁用）' }}</span>
                  </div>
                </div>
              </el-popover>
            </div>
            <el-input
              v-model="form.preScript"
              type="textarea"
              :rows="10"
              placeholder="// 前置脚本（Pre-request）· 使用 Postman(pm) 语法&#10;// 示例: pm.request.headers.add({ key:'Authorization', value:'Bearer '+pm.environment.get('token') })&#10;// pm.variables.set('ts', String(Date.now())); pm.request.url = pm.request.url + '?ts={{ts}}'"
              class="script-textarea"
            />
          </el-tab-pane>
          <el-tab-pane name="postScript">
            <template #label>
              <span class="tab-label-with-dot">后置脚本<span v-if="hasPostScript" class="tab-dot" /></span>
            </template>
            <div class="script-bar">
              <span class="script-hint">在收到响应后执行 · 支持 async/await</span>
              <el-button
                v-if="!form.postScript.trim()"
                size="small" link type="primary"
                @click="form.postScript = POST_SCRIPT_TEMPLATE"
              >插入模板</el-button>
              <el-button
                v-else
                size="small" link type="danger"
                @click="form.postScript = ''"
              >清空</el-button>
              <el-popover :width="240" trigger="click" v-model:visible="postVarVisible">
                <template #reference>
                  <el-button :icon="Coin" size="small" class="btn-var" title="插入环境变量（先聚焦脚本再点此）" @mousedown.prevent="captureFocusedInput" />
                </template>
                <div class="var-pop">
                  <div v-if="!activeVariables.length" class="var-pop-empty">当前环境暂无变量</div>
                  <div
                    v-for="v in activeVariables"
                    v-else
                    :key="v.key"
                    class="var-pop-item"
                    :class="{ off: !v.enabled }"
                    @click="insertVar(v.key); postVarVisible = false"
                  >
                    <span class="vp-k">&#123;&#123;{{ v.key }}&#125;&#125;</span>
                    <span class="vp-v">{{ v.enabled ? (v.value || '（空）') : '（已禁用）' }}</span>
                  </div>
                </div>
              </el-popover>
            </div>
            <el-input
              v-model="form.postScript"
              type="textarea"
              :rows="10"
              placeholder="// 后置脚本（Post-response）· 使用 Postman(pm) 语法&#10;// 示例: pm.environment.set('token', pm.response.json().token)&#10;// pm.test('状态码 200', () => pm.expect(pm.response.code).to.equal(200))"
              class="script-textarea"
            />
          </el-tab-pane>
        </el-tabs>

        <!-- 响应区 -->
        <div class="response-area">
          <div class="response-header">
            <span class="resp-label">响应</span>
            <template v-if="response">
              <el-tag :type="statusTagType" effect="dark" size="small">
                HTTP {{ response.status }} {{ response.statusText }}
              </el-tag>
              <el-tag v-if="businessCode?.isError" type="danger" effect="plain" size="small" class="biz-code-tag">
                业务 {{ businessCode.code }}
              </el-tag>
              <el-tag v-else-if="businessCode" type="success" effect="plain" size="small" class="biz-code-tag">
                业务 {{ businessCode.code }}
              </el-tag>
              <span class="meta">耗时 {{ response.timeMs }} ms</span>
              <span class="meta">大小 {{ formatSize(response.size) }}</span>
            </template>
            <template v-else-if="error">
              <el-tag type="danger" effect="dark" size="small">错误</el-tag>
              <span class="meta error-text">{{ error }}</span>
            </template>
            <template v-else>
              <span class="meta">尚未发送请求</span>
            </template>
          </div>

          <!-- 业务错误提示 -->
<!--          <div v-if="response && businessCode?.isError" class="biz-error-banner">-->
<!--            <el-icon :size="14"><WarningFilled /></el-icon>-->
<!--            <span class="biz-error-label">业务异常：</span>-->
<!--            <span class="biz-error-code">code={{ businessCode.code }}</span>-->
<!--            <span v-if="businessCode.msg" class="biz-error-msg">· {{ businessCode.msg }}</span>-->
<!--          </div>-->

          <el-tabs v-if="response" v-model="respTab" class="resp-tabs">
            <el-tab-pane label="Body" name="body">
              <div class="resp-body-bar">
                <el-radio-group v-model="respView" size="small">
                  <el-radio-button value="pretty">Pretty</el-radio-button>
                  <el-radio-button value="raw">Raw</el-radio-button>
                </el-radio-group>
                <el-button size="small" link type="primary" @click="copyResponseBody">复制</el-button>
              </div>
              <pre class="resp-body">{{ respView === 'pretty' ? prettyBody : response.body }}</pre>
            </el-tab-pane>
            <el-tab-pane :label="`响应头 (${response.headerCount})`" name="headers">
              <el-table :data="response.headers" size="small" stripe>
                <el-table-column prop="key" label="Header" width="240" />
                <el-table-column prop="value" label="值" show-overflow-tooltip />
              </el-table>
            </el-tab-pane>
            <el-tab-pane v-if="requestHeaders.length" :label="`请求头 (${requestHeaders.length})`" name="requestHeaders">
              <el-table :data="requestHeaders" size="small" stripe>
                <el-table-column prop="key" label="Header" width="240" />
                <el-table-column prop="value" label="值" show-overflow-tooltip />
              </el-table>
            </el-tab-pane>
            <!-- 请求详情 -->
            <el-tab-pane label="请求详情" name="requestDetail">
              <div class="req-detail-panel">
                <!-- 基本信息 -->
                <div class="req-detail-section">
                  <div class="req-detail-row">
                    <span class="req-detail-label">方法</span>
                    <el-tag :type="methodTagType(form.method)" effect="dark" size="small">{{ form.method }}</el-tag>
                  </div>
                  <div class="req-detail-row">
                    <span class="req-detail-label">URL</span>
                    <code class="req-detail-url">{{ resolvedUrl || '(未填写)' }}</code>
                  </div>
                  <div v-if="enabledParams.length && fullUrl" class="req-detail-row">
                    <span class="req-detail-label">完整 URL</span>
                    <code class="req-detail-url req-detail-url-full">{{ fullUrl }}</code>
                  </div>
                  <div v-if="contentTypeLabel" class="req-detail-row">
                    <span class="req-detail-label">Content-Type</span>
                    <code class="req-detail-code">{{ contentTypeLabel }}</code>
                  </div>
                  <div v-if="form.bodyType !== 'none' && form.body.trim()" class="req-detail-row">
                    <span class="req-detail-label">请求体</span>
                    <span class="meta">{{ form.bodyType }} · {{ formatSize(bodyByteSize) }}</span>
                  </div>
                </div>
                <!-- 查询参数 -->
                <div v-if="enabledParams.length" class="req-detail-section">
                  <div class="req-detail-subtitle">查询参数 ({{ enabledParams.length }})</div>
                  <div class="req-detail-kv-list">
                    <div v-for="p in enabledParams" :key="p.key" class="req-detail-kv-item">
                      <span class="req-detail-kv-key">{{ resolvedHeaderValue(p.key) }}</span>
                      <span class="req-detail-kv-eq">=</span>
                      <span class="req-detail-kv-val">{{ resolvedHeaderValue(p.value) }}</span>
                    </div>
                  </div>
                </div>
                <!-- 请求头 -->
                <div v-if="enabledHeaders.length" class="req-detail-section">
                  <div class="req-detail-subtitle">请求头 ({{ enabledHeaders.length }})</div>
                  <div class="req-detail-kv-list">
                    <div v-for="h in enabledHeaders" :key="h.key" class="req-detail-kv-item">
                      <span class="req-detail-kv-key">{{ resolvedHeaderValue(h.key) }}</span>
                      <span class="req-detail-kv-eq">:</span>
                      <span class="req-detail-kv-val">{{ resolvedHeaderValue(h.value) }}</span>
                    </div>
                  </div>
                </div>
                <!-- 表单字段 -->
                <div v-if="form.bodyType === 'form' && enabledFormBody.length" class="req-detail-section">
                  <div class="req-detail-subtitle">表单字段 ({{ enabledFormBody.length }})</div>
                  <div class="req-detail-kv-list">
                    <div v-for="f in enabledFormBody" :key="f.key" class="req-detail-kv-item">
                      <span class="req-detail-kv-key">{{ resolvedHeaderValue(f.key) }}</span>
                      <span class="req-detail-kv-eq">=</span>
                      <span class="req-detail-kv-val">{{ resolvedHeaderValue(f.value) }}</span>
                    </div>
                  </div>
                </div>
                <!-- 请求体预览 -->
                <div v-if="form.bodyType !== 'none' && form.body.trim() && form.bodyType !== 'form'" class="req-detail-section">
                  <div class="req-detail-subtitle">请求体预览</div>
                  <pre class="req-detail-body">{{ previewBodyContent }}</pre>
                </div>
              </div>
            </el-tab-pane>
            <el-tab-pane v-if="scriptLog.length" :label="`脚本日志 (${scriptLog.length})`" name="scriptLog">
              <div class="script-log-container">
                <div
                  v-for="(log, idx) in scriptLog"
                  :key="idx"
                  class="script-log-line"
                  :class="'log-' + log.type"
                >
                  <span class="log-prefix">{{ log.type === 'error' ? '✗' : log.type === 'warn' ? '⚠' : log.type === 'success' ? '✓' : '›' }}</span>
                  <span class="log-msg">{{ log.message }}</span>
                </div>
              </div>
            </el-tab-pane>
          </el-tabs>
        </div>

        </div><!-- end debug mode -->

        <!-- ====== 预览模式 ====== -->
        <div v-else-if="workMode === 'preview'" class="preview-panel">
          <!-- 请求摘要卡片 -->
          <div class="preview-section">
            <div class="preview-section-header">
              <el-icon><Document /></el-icon>
              <span>请求摘要</span>
              <el-tag :type="methodTagType(form.method)" effect="dark" size="small" style="margin-left: auto;">{{ form.method }}</el-tag>
            </div>
            <div class="preview-summary">
              <!-- 基础URL -->
              <div class="preview-summary-row">
                <span class="preview-label">URL</span>
                <code class="preview-url">{{ resolvedUrl || '(未填写)' }}</code>
              </div>
              <!-- 完整URL（含参数） -->
              <div v-if="enabledParams.length && fullUrl" class="preview-summary-row">
                <span class="preview-label">完整 URL</span>
                <code class="preview-url preview-url-full">{{ fullUrl }}</code>
              </div>
              <!-- Content-Type -->
              <div v-if="contentTypeLabel" class="preview-summary-row">
                <span class="preview-label">类型</span>
                <span class="preview-kv-count">{{ contentTypeLabel }}</span>
              </div>
              <!-- 请求头数量 -->
              <div v-if="enabledHeaders.length" class="preview-summary-row">
                <span class="preview-label">请求头</span>
                <span class="preview-kv-count">{{ enabledHeaders.length }} 个</span>
              </div>
              <!-- 查询参数数量 -->
              <div v-if="enabledParams.length" class="preview-summary-row">
                <span class="preview-label">参数</span>
                <span class="preview-kv-count">{{ enabledParams.length }} 个</span>
              </div>
              <!-- 请求体信息 -->
              <div v-if="form.bodyType !== 'none' && form.body.trim()" class="preview-summary-row">
                <span class="preview-label">请求体</span>
                <span class="preview-kv-count">{{ form.bodyType }} · {{ formatSize(bodyByteSize) }}</span>
              </div>
              <!-- form-urlencoded 参数 -->
              <div v-if="form.bodyType === 'form' && enabledFormBody.length" class="preview-summary-row">
                <span class="preview-label">表单</span>
                <span class="preview-kv-count">{{ enabledFormBody.length }} 个字段</span>
              </div>
            </div>
          </div>

          <!-- cURL 命令 -->
          <div class="preview-section">
            <div class="preview-section-header">
              <el-icon><Promotion /></el-icon>
              <span>cURL 命令</span>
              <el-button size="small" link type="primary" @click="copyCurl" class="preview-copy-btn">
                复制
              </el-button>
            </div>
            <pre class="preview-curl" @click="copyCurl" title="点击复制">{{ curlCommand }}</pre>
          </div>

          <!-- 请求头详情 -->
          <div v-if="enabledHeaders.length" class="preview-section">
            <div class="preview-section-header">
              <el-icon><InfoFilled /></el-icon>
              <span>请求头详情</span>
            </div>
            <div class="preview-headers-list">
              <div v-for="h in enabledHeaders" :key="h.key" class="preview-header-item">
                <span class="preview-header-key">{{ h.key }}</span>
                <span class="preview-header-sep">:</span>
                <span class="preview-header-val">{{ resolvedHeaderValue(h.value) }}</span>
              </div>
            </div>
          </div>

          <!-- 参数详情 -->
          <div v-if="enabledParams.length" class="preview-section">
            <div class="preview-section-header">
              <el-icon><Folder /></el-icon>
              <span>查询参数</span>
            </div>
            <div class="preview-params-list">
              <div v-for="p in enabledParams" :key="p.key" class="preview-param-item">
                <span class="preview-param-key">{{ p.key }}</span>
                <span class="preview-param-eq">=</span>
                <span class="preview-param-val">{{ resolvedHeaderValue(p.value) }}</span>
              </div>
            </div>
          </div>

          <!-- 请求体预览 -->
          <div v-if="form.bodyType !== 'none' && form.body.trim()" class="preview-section">
            <div class="preview-section-header">
              <el-icon><Document /></el-icon>
              <span>请求体预览</span>
              <span class="meta" style="margin-left: auto;">{{ formatSize(bodyByteSize) }}</span>
            </div>
            <pre class="preview-body-content">{{ previewBodyContent }}</pre>
          </div>

          <!-- 表单参数详情 -->
          <div v-if="form.bodyType === 'form' && enabledFormBody.length" class="preview-section">
            <div class="preview-section-header">
              <el-icon><Folder /></el-icon>
              <span>表单字段</span>
            </div>
            <div class="preview-params-list">
              <div v-for="f in enabledFormBody" :key="f.key" class="preview-param-item">
                <span class="preview-param-key">{{ resolvedHeaderValue(f.key) }}</span>
                <span class="preview-param-eq">=</span>
                <span class="preview-param-val">{{ resolvedHeaderValue(f.value) }}</span>
              </div>
            </div>
          </div>

          <!-- 响应摘要 -->
          <div v-if="response" class="preview-section preview-section--resp">
            <div class="preview-section-header">
              <el-icon><Star /></el-icon>
              <span>最近响应</span>
              <el-tag :type="statusTagType" effect="dark" size="small">HTTP {{ response.status }} {{ response.statusText }}</el-tag>
              <el-tag v-if="businessCode?.isError" type="danger" effect="plain" size="small" class="biz-code-tag">
                业务 {{ businessCode.code }}
              </el-tag>
              <el-tag v-else-if="businessCode" type="success" effect="plain" size="small" class="biz-code-tag">
                业务 {{ businessCode.code }}
              </el-tag>
              <span class="meta" style="margin-left: auto;">{{ response.timeMs }}ms · {{ formatSize(response.size) }}</span>
            </div>
            <!-- 业务错误提示 -->
<!--            <div v-if="businessCode?.isError" class="biz-error-banner">-->
<!--              <el-icon :size="14"><WarningFilled /></el-icon>-->
<!--              <span class="biz-error-label">业务异常：</span>-->
<!--              <span class="biz-error-code">code={{ businessCode.code }}</span>-->
<!--              <span v-if="businessCode.msg" class="biz-error-msg">· {{ businessCode.msg }}</span>-->
<!--            </div>-->
            <pre class="preview-body-content">{{ prettyBody }}</pre>
          </div>

          <!-- 响应头详情 -->
          <div v-if="response && resolvedRespHeaders.length" class="preview-section">
            <div class="preview-section-header">
              <el-icon><InfoFilled /></el-icon>
              <span>响应头</span>
              <span class="meta" style="margin-left: auto;">{{ resolvedRespHeaders.length }} 个</span>
            </div>
            <div class="preview-headers-list">
              <div v-for="h in resolvedRespHeaders" :key="h.key" class="preview-header-item">
                <span class="preview-header-key">{{ h.key }}</span>
                <span class="preview-header-sep">:</span>
                <span class="preview-header-val">{{ h.value }}</span>
              </div>
            </div>
          </div>
        </div>

        <!-- ====== 设计模式 ====== -->
        <div v-else-if="workMode === 'design'" class="design-panel">
          <!-- 设计模式说明 -->
          <div class="design-intro">
            <el-icon :size="18"><InfoFilled /></el-icon>
            <div>
              <div class="design-intro-title">接口设计 & Mock</div>
              <div class="design-intro-desc">编写接口文档 · 配置模拟响应 · 无需真实后端即可前端调试</div>
            </div>
          </div>

          <!-- 接口文档 (Markdown) -->
          <div class="design-doc-section">
            <div class="design-doc-tabs">
              <button
                class="design-doc-tab"
                :class="{ active: designDocTab === 'edit' }"
                @click="designDocTab = 'edit'"
              >
                <el-icon :size="13"><EditPen /></el-icon>
                编辑
              </button>
              <button
                class="design-doc-tab"
                :class="{ active: designDocTab === 'preview' }"
                @click="designDocTab = 'preview'"
              >
                <el-icon :size="13"><View /></el-icon>
                预览
              </button>
            </div>
            <div v-if="designDocTab === 'edit'" class="design-doc-editor">
              <el-input
                v-model="designDoc"
                type="textarea"
                :rows="16"
                placeholder="支持 Markdown 格式&#10;&#10;# 标题&#10;## 子标题&#10;**粗体** *斜体*&#10;- 列表项&#10;```json&#10;{ }&#10;```&#10;| 表头 | 表头 |&#10;| --- | --- |"
                class="design-doc-textarea"
                resize="vertical"
              />
            </div>
            <div
              v-else
              class="design-doc-preview markdown-body"
              v-html="renderedDoc"
            />
          </div>

          <!-- Mock 响应列表 -->
          <div class="design-mock-list">
            <div
              v-for="(mock, idx) in mockResponses"
              :key="mock.id"
              class="design-mock-item"
              :class="{ active: mock.isActive }"
            >
              <div class="mock-item-header">
                <el-switch
                  v-model="mock.isActive"
                  size="small"
                  @change="onMockToggle(idx)"
                />
                <el-input
                  v-model="mock.name"
                  size="small"
                  placeholder="场景名称"
                  class="mock-name-input"
                />
                <el-select
                  v-model="mock.statusCode"
                  size="small"
                  class="mock-status-select"
                  @change="(v: number) => onMockStatusChange(mock, v)"
                >
                  <el-option
                    v-for="opt in MOCK_STATUS_OPTIONS"
                    :key="opt.value"
                    :label="opt.label"
                    :value="opt.value"
                  />
                </el-select>
                <el-button
                  link
                  type="danger"
                  size="small"
                  @click="removeMockResponse(idx)"
                >删除</el-button>
              </div>

              <div class="mock-item-body">
                <div class="mock-field">
                  <label>延迟 (ms)</label>
                  <el-input-number
                    v-model="mock.delay"
                    :min="0"
                    :max="10000"
                    :step="100"
                    size="small"
                    controls-position="right"
                  />
                </div>
                <div class="mock-field">
                  <label>响应头 (JSON)</label>
                  <el-input
                    v-model="mock.headers"
                    type="textarea"
                    :rows="3"
                    placeholder='{"Content-Type": "application/json"}'
                    class="mock-textarea"
                  />
                </div>
                <div class="mock-field">
                  <label>响应体</label>
                  <el-input
                    v-model="mock.body"
                    type="textarea"
                    :rows="6"
                    placeholder='{ "code": 0, "data": {}, "message": "success" }'
                    class="mock-textarea"
                  />
                </div>
              </div>

              <!-- 快速发送 Mock -->
              <div class="mock-item-actions">
                <el-button
                  size="small"
                  type="primary"
                  :icon="Position"
                  @click="sendMockResponse(idx)"
                >
                  模拟发送
                </el-button>
                <el-button
                  size="small"
                  @click="formatMockBody(mock)"
                >格式化 JSON</el-button>
              </div>
            </div>
          </div>

          <el-button
            class="design-add-btn"
            @click="addMockResponse"
          >
            + 添加 Mock 场景
          </el-button>

          <!-- 设计模式下的响应展示 -->
          <div v-if="response && mockSendFromDesign" class="design-result">
            <div class="design-result-header">
              <span>模拟响应结果</span>
              <el-tag :type="statusTagType" effect="dark" size="small">
                {{ response.status }} {{ response.statusText }}
              </el-tag>
              <span class="meta">{{ response.timeMs }}ms</span>
            </div>
            <pre class="preview-body-content">{{ prettyBody }}</pre>
          </div>
        </div>

        <div v-else-if="workMode === 'websocket'" class="ws-mode-panel">
          <WebSocketTester />
        </div>

        </template>

        <!-- 无任何页签时的空状态引导 -->
        <template v-else>
          <div class="empty-tabs">
            <el-icon :size="40" class="et-icon"><Plus /></el-icon>
            <div class="et-title">还没有打开任何请求</div>
            <div class="et-desc">点击下方按钮新建一个请求，或从左侧分类树 / 历史记录打开已有接口</div>
            <el-button type="primary" :icon="Plus" @click="addTab">新建请求</el-button>
          </div>
        </template>
      </section>

    </main>

    <!-- 版本历史抽屉 -->
    <VersionHistoryDrawer
      v-model:visible="versionDrawerVisible"
      :request-id="currentRequestId"
      @restored="onVersionRestored"
    />

    <!-- Mock 服务抽屉 -->
    <el-drawer v-model="mockDrawerVisible" title="本地 Mock 服务" size="640px" direction="rtl">
      <MockManager />
    </el-drawer>

    <!-- 历史滑出面板 -->
    <aside class="history-slide-panel" :class="{ 'history-slide--open': showHistory }">
      <div class="history-panel-inner">
        <div class="history-header">
          <span class="history-title">历史记录</span>
          <div class="history-header-actions">
            <el-button
              v-if="history.length && canWrite"
              size="small"
              link
              type="danger"
              @click="clearHistory"
            >清空</el-button>
            <el-button size="small" link @click="closeHistory" title="关闭">
              <el-icon><View /></el-icon>
            </el-button>
          </div>
        </div>
        <el-scrollbar v-if="filteredHistory.length" class="history-scroll">
          <div
            v-for="(h, idx) in filteredHistory"
            :key="h.id"
            class="history-item"
            :class="{ active: activeHistoryId === h.id }"
            @click="loadHistory(h)"
          >
            <div class="hi-top">
              <el-tag size="small" :type="methodTagType(h.method)" effect="plain">{{ h.method }}</el-tag>
              <span class="hi-url" :title="h.url">{{ h.url || '(empty)' }}</span>
            </div>
            <div class="hi-bot">
              <el-tag
                v-if="h.status"
                size="small"
                :type="statusTagTypeOf(h.status)"
                effect="dark"
              >{{ h.status }}</el-tag>
              <span v-else class="hi-status-empty">未发送</span>
              <el-tag v-if="h.categoryId" size="small" type="info" effect="plain" class="hi-cat">
                {{ getCategoryName(h.categoryId || '') }}
              </el-tag>
              <span class="hi-time">{{ formatTime(parseServerTime(h.createTime)) }}</span>
              <el-button
                v-if="canWrite"
                link
                type="danger"
                size="small"
                class="hi-del"
                @click.stop="removeHistory(idx)"
              >删除</el-button>
            </div>
          </div>
        </el-scrollbar>
        <EmptyState v-else compact title="暂无历史" />
      </div>
    </aside>

    <!-- 环境变量提示（右下角悬浮） -->
    <transition name="var-fade">
      <div v-if="activeEnv && activeVariables.length" class="var-float">
        <div class="var-hint-bar">
          <div class="var-hint-header">
            <el-icon><InfoFilled /></el-icon>
            <span>{{ activeEnv.name }} · {{ activeVariables.filter(v => v.enabled).length }} 个变量</span>
            <el-button size="small" link @click="showVars = !showVars">
              {{ showVars ? '收起' : '展开' }}
            </el-button>
          </div>
          <div v-if="showVars" class="var-hint-tip">点击变量名即可插入到当前光标位置</div>
          <div v-if="showVars" class="var-hint-tags">
            <el-tooltip
              v-for="v in activeVariables.filter(x => x.enabled)"
              :key="v.id"
              :content="v.value"
              placement="top"
              :show-after="300"
            >
              <el-tag
                size="small"
                class="var-tag"
                @mousedown="captureFocusedInput"
                @click="insertVar(v.key)"
              >
                &#123;&#123;{{ v.key }}&#125;&#125;
              </el-tag>
            </el-tooltip>
          </div>
        </div>
      </div>
    </transition>
  </AppLayout>
</template>

<script setup lang="ts">
import EmptyState from '@/components/EmptyState.vue'
import { computed, onMounted, onUnmounted, reactive, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Promotion, Position, Star, Setting, EditPen, InfoFilled, Document, Folder, Clock, View, SetUp, Monitor, Cloudy, Connection, Switch, FolderOpened, Plus, Coin } from '@element-plus/icons-vue'
import { marked } from 'marked'
import DOMPurify from 'dompurify'
import KeyValueEditor from '@/components/KeyValueEditor.vue'
import CategoryTree from '@/components/CategoryTree.vue'
import WebSocketTester from '@/components/WebSocketTester.vue'
import VersionHistoryDrawer from '@/components/VersionHistoryDrawer.vue'
import AppLayout from '@/layouts/AppLayout.vue'
import MockManager from '@/components/MockManager.vue'
import EnvironmentManager from '@/components/EnvironmentManager.vue'
import { useCategories } from '@/composables/useCategories'
import { useProjects } from '@/composables/useProjects'
import type { Category } from '@/composables/useCategories'
import { useSavedRequests } from '@/composables/useSavedRequests'
import { useEnvironments } from '@/composables/useEnvironments'
import type { SavedRequest } from '@/composables/useSavedRequests'
import type { Method, BodyType, KV, ResponseInfo, HistoryItem, WorkMode, MockResponse } from '@/types'
import { METHODS, emptyKV, MOCK_STATUS_OPTIONS } from '@/types'
import {
  looksLikeJson, statusTagTypeOf, methodTagType,
  formatSize, formatTime, parseServerTime, kvToObject, shortenUrl, stripJsonComments, uid,
} from '@/utils'
import { useTabs, createDefaultTabState } from '@/composables/useTabs'
import { useScriptEngine, PRE_SCRIPT_TEMPLATE, POST_SCRIPT_TEMPLATE } from '@/composables/useScriptEngine'
import { useHistory } from '@/composables/useHistory'
import { useSidebar } from '@/composables/useSidebar'
import { buildUrl, buildBody, invokeHttpRequest, invokeHttpRequestStream } from '@/composables/useHttpRequest'
import { canWrite, loadTeams, loadCurrentTeamMembers } from '@/composables/useTeams'
import { useDataMode } from '@/composables/useDataMode'
// ====== Composables ======
const {
  categories,
  selectedCategoryId,
  getChildren,
} = useCategories()

const {
  savedRequests,
  saveRequest,
  updateRequest,
  deleteRequest,
  load: loadRequests,
} = useSavedRequests()
/* ============ 数据模式（在线/离线隔离） ============ */
const { dataMode } = useDataMode()
// ====== 项目（从路由参数传入，锁定当前工作项目）======
const props = defineProps<{ projectId: string | null }>()
const router = useRouter()
const { projects, selectProject, clearProject, suppressAutoEnter, load: loadProjects } = useProjects()

const projectName = computed(() => {
  if (!props.projectId) return ''
  return projects.find(c => c.id === props.projectId)?.name || '未命名项目'
})

// 路由携带项目时同步到全局选择状态（持久化），刷新后不会丢失
watch(
  () => props.projectId,
  (id, prev) => {
    // 同一组件实例内路由参数换项目（/project/A → /project/B）时不会重新 setup，
    // 需在此关闭上一个项目已打开的页签。首次挂载 prev 为空，故不会触发
    // （也避免在 useTabs() 之前访问 closeAllTabs 造成暂时性死区）。
    if (prev && id !== prev) closeAllTabs()
    if (id) {
      // 同步带上当前用户在该项目中的角色，否则 selectedProjectRole 会被重置为 null，
      // 导致项目成员管理的「所有者/管理员」权限判定失效（添加/移除成员入口消失）。
      const p = projects.find(c => c.id === id)
      selectProject(id, p?.currentUserRole)
    }
  },
  { immediate: true },
)

// 切换在线/离线：重新拉取对应模式物理库的项目列表，并直接回到项目选择页。
// 无论切换后该模式是否有项目，都回到项目页让用户重新选择/创建，
// 避免在线模式无项目时停留在调试页无法返回。
// flush: 'post' 确保只在 DOM 更新后的变化触发，避免初始挂载时误执行 clearProject + 跳转。
watch(dataMode, async () => {
  await loadProjects()
  clearProject()
  // 页签归属项目，切模式后项目上下文已失效：关闭全部页签，避免带到另一模式的项目里
  closeAllTabs()
  // 标记本次为切模式回项目页，抑制 ProjectSelectView 的"自动进入上次项目"逻辑，
  // 否则会立即又被跳回接口页。
  suppressAutoEnter.value = true
  router.replace('/')
}, { flush: 'post' })

// 切换项目：关闭当前项目已打开的全部接口页签，清除持久化并回到项目选择页
function switchProject() {
  closeAllTabs()
  clearProject()
  // 抑制 ProjectSelectView 的自动恢复逻辑，否则回到项目页后会立即又被跳回接口页
  suppressAutoEnter.value = true
  router.push('/')
}

function getCategoryName(id: string): string {
  const cat = categories.find(c => c.id === id)
  return cat ? cat.name : ''
}

function getCategoryPath(id: string): Category[] {
  const path: Category[] = []
  let cur: Category | undefined = categories.find(c => c.id === id)
  const guard = new Set<string>()
  while (cur && !guard.has(cur.id)) {
    guard.add(cur.id)
    path.unshift(cur)
    cur = cur.parentId ? categories.find(c => c.id === cur!.parentId as string) : undefined
  }
  return path
}

const {
  environments,
  envGroups,
  activeVariables,
  activeEnv,
  load: loadEnvironments,
  activateEnv,
  resolveVariables,
  saveVariable,
  updateVariable,
} = useEnvironments()

// ====== 国际化 ======
import { useI18n } from 'vue-i18n'
const { t } = useI18n()

// 环境按分组归类
const groupedEnvs = computed(() => {
  const grouped: { group: { id: string; name: string }; envs: typeof environments.value }[] = []
  const ungrouped: typeof environments.value = []
  const groupMap = new Map(envGroups.value.map(g => [g.id, g]))

  for (const env of environments.value) {
    if (env.groupId && groupMap.has(env.groupId)) {
      const g = groupMap.get(env.groupId)!
      let entry = grouped.find(x => x.group.id === g.id)
      if (!entry) {
        entry = { group: g, envs: [] }
        grouped.push(entry)
      }
      entry.envs.push(env)
    } else {
      ungrouped.push(env)
    }
  }
  grouped.sort((a, b) => {
    const ga = envGroups.value.find(g => g.id === a.group.id)
    const gb = envGroups.value.find(g => g.id === b.group.id)
    return (ga?.sortOrder ?? 0) - (gb?.sortOrder ?? 0)
  })
  return { grouped, ungrouped }
})

const {
  tabs, activeTabId,
  editingTabId, editingTabTitle,
  tabInputRef, tabListRef,
  currentTabTitle,
  snapshotCurrentTab, saveCurrentTabState, persistTabs,
  startEditTabTitle, finishEditTabTitle, cancelEditTabTitle,
  closeAllTabs,
} = useTabs(() => props.projectId ?? '')

const { scriptLog, scriptVars, runScript } = useScriptEngine(activeVariables, activeEnv, updateVariable, saveVariable)

const {
  history, activeHistoryId, filteredHistory,
  pushHistory, removeHistory, clearHistory, loadPersistedHistory,
} = useHistory(selectedCategoryId)

const { sidebarRef, sidebarWidth, startSidebarResize } = useSidebar()

// ====== 环境管理 ======
const envManagerRef = ref<InstanceType<typeof EnvironmentManager> | null>(null)
function handleEnvSwitch(envId: string) {
  if (envId) activateEnv(envId)
}
function openEnvManager() {
  envManagerRef.value?.open()
}

const showVars = ref(false)

// 记录点击变量前最后聚焦的输入框，用于「插入到光标处」
let lastFocusedInputEl: HTMLInputElement | HTMLTextAreaElement | null = null
function captureFocusedInput() {
  const el = document.activeElement
  if (el && (el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement)) {
    lastFocusedInputEl = el
  } else {
    lastFocusedInputEl = null
  }
}

// 将文本插入到聚焦输入框的光标位置，并同步 el-input 的 v-model
function insertAtCursor(text: string): boolean {
  const el = lastFocusedInputEl
  if (!el || !el.isConnected) return false
  const start = el.selectionStart ?? el.value.length
  const end = el.selectionEnd ?? el.value.length
  el.focus()
  el.setRangeText(text, start, end, 'end')
  el.dispatchEvent(new Event('input', { bubbles: true }))
  return true
}

function insertVar(varName: string) {
  const syntax = `{{${varName}}}`
  if (insertAtCursor(syntax)) {
    ElMessage.success(`已插入 ${syntax}`)
    return
  }
  // 未聚焦输入框时回退为复制
  navigator.clipboard.writeText(syntax).then(
    () => ElMessage.success(`已复制 ${syntax}`),
    () => ElMessage.error('复制失败'),
  )
}

// 文本框旁「插入变量」弹层的显示状态（点选变量时直接关闭对应弹层）
const urlVarVisible = ref(false)
const bodyVarVisible = ref(false)
const preVarVisible = ref(false)
const postVarVisible = ref(false)

// ====== 工作模式 ======
/** 首次进入页面时，中间区域默认展示系统说明页；用户开始操作后进入调试界面 */
// 首次进入展示系统说明页；若本地恢复了上次打开的页签（刷新场景），则直接进入工作区
const showWelcome = ref(tabs.value.length === 0)

function enterWorkspace() {
  showWelcome.value = false
}

// 页签被清空时回到系统说明页。
// 模板是 `v-if="showWelcome"` / `v-else-if="tabs.length"`：只清空页签而不置位 showWelcome
// 会让两者都不匹配、中间区域变成空白。触发场景：切换项目关闭全部页签、或关掉最后一个页签。
watch(() => tabs.value.length, (len) => {
  if (len === 0) showWelcome.value = true
})

const workMode = ref<WorkMode>('debug')
const workModes = [
  { key: 'debug' as WorkMode, label: '调试', icon: Monitor },
  { key: 'preview' as WorkMode, label: '预览', icon: View },
  { key: 'design' as WorkMode, label: '设计', icon: SetUp },
  { key: 'websocket' as WorkMode, label: 'WebSocket', icon: Connection },
]

// ====== 系统说明页（首次进入默认展示） ======
const welcomeFeatures = [
  { icon: Monitor, title: '多标签页调试', desc: '同时打开多个接口互不干扰，支持双击重命名与右键快捷管理' },
  { icon: Cloudy, title: '环境变量', desc: '多套环境一键切换，{{变量}} 语法贯穿 URL、Header、Body 与脚本' },
  { icon: EditPen, title: '前后置脚本', desc: '内置 CryptoJS 与 test/assert 工具，可改写请求、断言响应' },
  { icon: Folder, title: '接口管理', desc: '分类树保存常用接口，支持拖拽整理与 JSON 批量导入导出' },
  { icon: SetUp, title: '设计模式', desc: '编写 Markdown 接口文档，配置 Mock 场景即可无需后端联调' },
  { icon: Connection, title: 'WebSocket 调试', desc: '内置 WebSocket 调试器，实时收发与查看连接消息' },
  { icon: FolderOpened, title: '工作区与云同步', desc: '多工作区隔离项目数据，支持团队协作与权限控制' },
  { icon: View, title: '预览模式', desc: '查看解析后的 URL、参数与请求头，一键生成 cURL 命令' },
]

const welcomeSteps = [
  { title: '新建请求', desc: '点击下方「开始调试」，填写请求方法与 URL' },
  { title: '配置环境', desc: '在顶部切换环境，用 {{变量}} 引用环境变量' },
  { title: '发送调试', desc: '发送请求，查看响应、耗时与脚本日志' },
  { title: '保存沉淀', desc: '按 Ctrl+S 保存接口，或导出 JSON 分享给团队' },
]

function startDebugging() {
  addTab()
}

// ====== 请求表单（当前活跃页签的响应式状态） ======
const form = reactive({
  method: 'GET' as Method,
  url: '',
  params: [emptyKV()] as KV[],
  headers: [emptyKV()] as KV[],
  bodyType: 'none' as BodyType,
  body: '',
  formBody: [emptyKV()] as KV[],
  categoryId: undefined as string | undefined,
  preScript: '',
  postScript: '',
})

// 表单（含 body）实时变化即持久化 tabs 状态，刷新页面后可恢复正在编辑的内容（含请求体）
watch(form, () => {
  saveCurrentTabStateWrapper()
  persistTabs()
}, { deep: true })

const reqTab = ref<'params' | 'headers' | 'body' | 'preScript' | 'postScript'>('params')
const respTab = ref<'body' | 'headers' | 'requestHeaders' | 'requestDetail' | 'scriptLog'>('body')
const respView = ref<'pretty' | 'raw'>('pretty')
const requestHeaders = ref<{ key: string; value: string }[]>([])
const loading = ref(false)
const response = ref<ResponseInfo | null>(null)
/** SSE 流式发送开关 */
const streamMode = ref(false)
const error = ref<string>('')
const currentRequestId = ref<string>('')
const versionDrawerVisible = ref(false)
const mockDrawerVisible = ref(false)


function openVersionHistory() {
  if (!currentRequestId.value) return
  versionDrawerVisible.value = true
}

/**
 * 版本回退完成：回退只改了本地库内容，列表与已打开页签仍持有旧值。
 * 先刷新列表，再把恢复后的内容重新载入当前表单，否则界面上看不出任何变化。
 */
async function onVersionRestored(restored: SavedRequest) {
  await loadRequests()
  const fresh = savedRequests.value.find(r => r.id === restored.id) ?? restored
  loadSavedRequest(fresh)
}

const currentRequestName = computed(() => {
  if (!currentRequestId.value) return ''
  const r = savedRequests.value.find(r => r.id === currentRequestId.value)
  return r?.name ?? ''
})

// 编辑中请求的分类面包屑路径
const editingBreadcrumb = computed(() => {
  if (!currentRequestId.value) return []
  const req = savedRequests.value.find(r => r.id === currentRequestId.value)
  if (!req?.categoryId) return []
  return getCategoryPath(req.categoryId).map(c => c.name)
})

// ====== 历史显示/隐藏 ======
const HISTORY_KEY = 'canghai-show-history'
const showHistory = ref(localStorage.getItem(HISTORY_KEY) === '1')
function toggleHistory() {
  showHistory.value = !showHistory.value
  localStorage.setItem(HISTORY_KEY, showHistory.value ? '1' : '0')
}
// 面板内关闭按钮：同步持久化，避免下次进入又读回旧的 '1'
function closeHistory() {
  showHistory.value = false
  localStorage.setItem(HISTORY_KEY, '0')
}

// ====== 团队 ======

// ====== 布局样式 ======
const contentGridStyle = computed(() => {
  const cols = showHistory.value
    ? `${sidebarWidth.value}px 1fr 300px`
    : `${sidebarWidth.value}px 1fr`
  return { gridTemplateColumns: cols }
})

// ====== 计算属性 ======
const methodAllowsBody = computed(() => !['GET', 'HEAD'].includes(form.method))
const statusTagType = computed(() => statusTagTypeOf(response.value?.status))

const hasParams = computed(() => form.params.some(p => p.key.trim()))
const hasHeaders = computed(() => form.headers.some(h => h.key.trim()))
const hasBody = computed(() => {
  if (form.bodyType === 'none') return false
  if (form.bodyType === 'form') return form.formBody.some(f => f.key.trim())
  return form.body.trim().length > 0
})
const hasPreScript = computed(() => form.preScript.trim().length > 0)
const hasPostScript = computed(() => form.postScript.trim().length > 0)

const prettyBody = computed(() => {
  if (!response.value) return ''
  const ct = response.value.contentType.toLowerCase()
  const body = response.value.body
  if (!body) return ''
  if (ct.includes('json') || looksLikeJson(body)) {
    try { return JSON.stringify(JSON.parse(body), null, 2) } catch { return body }
  }
  return body
})

// ====== 业务状态码检测 ======
interface BusinessCode {
  code: number | string
  msg?: string
  isError: boolean
}

const businessCode = computed<BusinessCode | null>(() => {
  if (!response.value) return null
  const ct = response.value.contentType.toLowerCase()
  const body = response.value.body
  if (!body) return null
  if (!(ct.includes('json') || looksLikeJson(body))) return null
  try {
    const parsed = JSON.parse(body)
    if (typeof parsed !== 'object' || parsed === null) return null
    // 尝试常见的业务状态码字段
    const code = parsed.code ?? parsed.errcode ?? parsed.status ?? parsed.resultCode
    if (code === undefined || code === null) return null
    const msg = parsed.msg ?? parsed.message ?? parsed.errmsg ?? parsed.info
    const numCode = typeof code === 'number' ? code : parseInt(code, 10)
    const isError = !isNaN(numCode) && numCode !== 0 && numCode !== 200
    return { code, msg, isError }
  } catch {
    return null
  }
})

function buildTreeData(parentId: string | null): any[] {
  return getChildren(parentId).map(c => ({
    id: c.id,
    name: c.name,
    children: buildTreeData(c.id),
  }))
}
const categoryTreeData = computed(() => buildTreeData(null))

// ====== 快照/恢复辅助 ======
function getFormGetters() {
  return {
    form,
    reqTab: reqTab.value,
    respTab: respTab.value,
    respView: respView.value,
    response: response.value,
    requestHeaders: requestHeaders.value,
    error: error.value,
    loading: loading.value,
    currentRequestId: currentRequestId.value,
    scriptLog: scriptLog.value,
  }
}

function restoreTab(tab: ReturnType<typeof snapshotCurrentTab>) {
  form.method = tab.form.method
  form.url = tab.form.url
  form.params = JSON.parse(JSON.stringify(tab.form.params))
  form.headers = JSON.parse(JSON.stringify(tab.form.headers))
  form.bodyType = tab.form.bodyType
  form.body = tab.form.body
  form.formBody = JSON.parse(JSON.stringify(tab.form.formBody))
  form.categoryId = tab.form.categoryId
  form.preScript = tab.form.preScript
  form.postScript = tab.form.postScript
  reqTab.value = tab.reqTab
  respTab.value = tab.respTab
  respView.value = tab.respView
  response.value = tab.response
  error.value = tab.error
  loading.value = tab.loading
  currentRequestId.value = tab.currentRequestId
  scriptLog.value = [...tab.scriptLog]
  requestHeaders.value = [...tab.requestHeaders]
}

function saveCurrentTabStateWrapper() {
  saveCurrentTabState(getFormGetters())
}

// ====== 页签操作 ======
function switchTab(tabId: string) {
  enterWorkspace()
  if (tabId === activeTabId.value) return
  saveCurrentTabStateWrapper()
  activeTabId.value = tabId
  activeHistoryId.value = ''
  const tab = tabs.value.find(t => t.id === tabId)
  if (tab) restoreTab(tab)
}

function addTab() {
  enterWorkspace()
  saveCurrentTabStateWrapper()
  const newTab = createDefaultTabState()
  // 新建请求默认归属左侧当前选中的接口分类（选中「全部接口」时为 null → 空串）
  newTab.form.categoryId = selectedCategoryId.value ?? ''
  tabs.value.push(newTab)
  activeTabId.value = newTab.id
  restoreTab(newTab)
}

function closeTab(tabId: string) {
  if (tabs.value.length <= 0) return
  const idx = tabs.value.findIndex(t => t.id === tabId)
  if (idx < 0) return
  tabs.value.splice(idx, 1)
  if (activeTabId.value === tabId) {
    // 关掉最后一个页签：清空活跃 id，由 tabs.length 的 watch 置回系统说明页。
    // （原实现会走到 tabs.value[-1].id，直接抛 TypeError）
    if (tabs.value.length === 0) {
      activeTabId.value = ''
      activeHistoryId.value = ''
      return
    }
    const newIdx = Math.min(idx, tabs.value.length - 1)
    activeTabId.value = tabs.value[newIdx].id
    activeHistoryId.value = ''
    restoreTab(tabs.value[newIdx])
  }
}

// ====== 页签右键菜单 ======
const tabContextMenu = reactive({ visible: false, x: 0, y: 0, tabId: '' })

function openTabContextMenu(e: MouseEvent, tabId: string) {
  tabContextMenu.x = e.clientX
  tabContextMenu.y = e.clientY
  tabContextMenu.tabId = tabId
  tabContextMenu.visible = true
  // 点击其他区域关闭菜单
  const closeMenu = () => {
    tabContextMenu.visible = false
    window.removeEventListener('click', closeMenu)
  }
  setTimeout(() => window.addEventListener('click', closeMenu), 0)
}

const hasTabsToRight = computed(() => {
  const idx = tabs.value.findIndex(t => t.id === tabContextMenu.tabId)
  return idx >= 0 && idx < tabs.value.length - 1
})

function ctxCloseTab() {
  closeTab(tabContextMenu.tabId)
}
function ctxCloseOthers() {
  if (tabs.value.length <= 1) return
  const keepId = tabContextMenu.tabId
  const keepTab = tabs.value.find(t => t.id === keepId)
  if (!keepTab) return
  // 先切换到保留的页签
  if (activeTabId.value !== keepId) {
    saveCurrentTabStateWrapper()
    activeTabId.value = keepId
    restoreTab(keepTab)
  }
  tabs.value = [keepTab]
}
function ctxCloseAll() {
  if (tabs.value.length <= 1) return
  const newTab = createDefaultTabState()
  tabs.value = [newTab]
  activeTabId.value = newTab.id
  activeHistoryId.value = ''
  restoreTab(newTab)
}
function ctxCloseToRight() {
  const idx = tabs.value.findIndex(t => t.id === tabContextMenu.tabId)
  if (idx < 0 || idx >= tabs.value.length - 1) return
  // 如果活跃页签在右侧，切换到目标页签
  const activeIdx = tabs.value.findIndex(t => t.id === activeTabId.value)
  if (activeIdx > idx) {
    saveCurrentTabStateWrapper()
    activeTabId.value = tabContextMenu.tabId
    restoreTab(tabs.value[idx])
  }
  tabs.value.splice(idx + 1)
}
function ctxDuplicateTab() {
  const srcTab = tabs.value.find(t => t.id === tabContextMenu.tabId)
  if (!srcTab) return
  saveCurrentTabStateWrapper()
  const newTab = createDefaultTabState(srcTab.title)
  newTab.form = JSON.parse(JSON.stringify(srcTab.form))
  tabs.value.push(newTab)
  activeTabId.value = newTab.id
  restoreTab(newTab)
}

// ====== 工具函数（视图层专用） ======
function formatJsonBody() {
  if (!form.body.trim()) return
  try {
    const cleaned = stripJsonComments(form.body).trim()
    form.body = JSON.stringify(JSON.parse(cleaned), null, 2)
  } catch {
    ElMessage.warning('请求体不是合法 JSON')
  }
}

// ====== 发送请求 ======
async function sendRequest() {
  const sendingTabId = activeTabId.value
  error.value = ''
  scriptLog.value = []
  requestHeaders.value = []

  // Auto-update tab title if still default
  const curTab = tabs.value.find(t => t.id === sendingTabId)
  if (curTab && (!curTab.title || curTab.title === '新请求') && form.url) {
    curTab.title = `${form.method} ${shortenUrl(form.url)}`
  }

  // Build initial request context for pre-script
  const reqCtx: Record<string, any> = {
    method: form.method,
    url: form.url,
    params: Object.fromEntries(
      form.params.filter(p => p.enabled && p.key).map(p => [p.key, p.value])
    ),
    headers: Object.fromEntries(
      form.headers.filter(h => h.enabled && h.key).map(h => [h.key, h.value])
    ),
    body: form.body,
    bodyType: form.bodyType,
  }

  // Run pre-request script
  if (form.preScript?.trim()) {
    const preLogs = await runScript(form.preScript, reqCtx, null)
    scriptLog.value.push(...preLogs)
    form.method = (reqCtx.method as Method) || form.method
    form.url = reqCtx.url || form.url
    if (reqCtx.body != null) form.body = reqCtx.body
    if (reqCtx.bodyType) form.bodyType = reqCtx.bodyType as BodyType
    // 同步脚本修改的 params 回表单
    if (reqCtx.params && typeof reqCtx.params === 'object') {
      const paramEntries = Object.entries(reqCtx.params as Record<string, string>)
      if (paramEntries.length) {
        form.params = paramEntries.map(([key, value]) => ({ key, value, enabled: true }))
      }
    }
    if (reqCtx.headers && typeof reqCtx.headers === 'object') {
      const headerEntries = Object.entries(reqCtx.headers as Record<string, string>)
      if (headerEntries.length) {
        form.headers = headerEntries.map(([key, value]) => ({ key, value, enabled: true }))
      }
    }
  }

  // 创建本地解析器：优先使用脚本内存变量
  const localResolve = (text: string): string => {
    if (!text) return text
    return text.replace(/\{\{\s*([\w$.]+)\s*\}\}/g, (_, key: string) => {
      if (scriptVars.has(key)) return scriptVars.get(key)!
      return resolveVariables(`{{${key}}}`)
    })
  }
  const localResolveKv = (arr: { key: string; value: string; enabled: boolean }[]) => {
    return arr.map(item => ({
      ...item,
      key: localResolve(item.key),
      value: localResolve(item.value),
    }))
  }

  let url: string
  try {
    const origUrl = form.url
    form.url = localResolve(form.url)
    const resolvedParams = localResolveKv(form.params)
    const origParams = form.params
    form.params = resolvedParams
    url = buildUrl(form.url, form.params)
    form.url = origUrl
    form.params = origParams
  } catch (e) {
    ElMessage.warning((e as Error).message)
    return
  }

  // resolve env vars in body
  const resolvedBody = localResolve(form.body)
  const origBody = form.body
  form.body = resolvedBody
  const resolvedFormBody = localResolveKv(form.formBody)
  const origFormBody = form.formBody
  form.formBody = resolvedFormBody
  const { body, contentType } = buildBody(form.method, form.bodyType, form.body, form.formBody)
  form.body = origBody
  form.formBody = origFormBody

  const resolvedHeaders = localResolveKv(form.headers)
  const userHeaders = kvToObject(resolvedHeaders)
  const finalHeaders: Record<string, string> = { ...userHeaders }
  if (contentType && !Object.keys(finalHeaders).some(k => k.toLowerCase() === 'content-type')) {
    finalHeaders['Content-Type'] = contentType
  }
  if (streamMode.value && !Object.keys(finalHeaders).some(k => k.toLowerCase() === 'accept')) {
    finalHeaders['Accept'] = 'text/event-stream'
  }
  const sentHeaders = Object.entries(finalHeaders).map(([key, value]) => ({ key, value }))

  loading.value = true
  response.value = null
  try {
    let info: ResponseInfo
    const stillActive = activeTabId.value === sendingTabId
    const targetTab = stillActive ? null : tabs.value.find(t => t.id === sendingTabId)

    if (streamMode.value) {
      // 流式 / SSE 模式：先建占位响应，再逐块追加
      const placeholder: ResponseInfo = {
        status: 0,
        statusText: '',
        timeMs: 0,
        size: 0,
        headers: [],
        headerCount: 0,
        body: '',
        contentType: 'text/event-stream',
      }
      if (stillActive) {
        response.value = placeholder
        respTab.value = 'body'
      } else if (targetTab) {
        targetTab.response = placeholder
        targetTab.respTab = 'body'
      }
      info = await invokeHttpRequestStream({
        method: form.method,
        url,
        headers: finalHeaders,
        body,
        onChunk: (text, done) => {
          const acc = (stillActive ? response.value : targetTab?.response)
          if (acc) {
            acc.body += text
            if (done) {
              if (stillActive) requestHeaders.value = sentHeaders
              else if (targetTab) targetTab.requestHeaders = sentHeaders
            }
          }
        },
      })
    } else {
      info = await invokeHttpRequest(form.method, url, finalHeaders, body)
    }

    // Run post-request script
    const postScriptText = stillActive ? form.postScript : (targetTab?.form.postScript ?? '')
    if (postScriptText?.trim()) {
      const postReqCtx: Record<string, any> = {
        method: stillActive ? form.method : (targetTab?.form.method ?? form.method),
        url: stillActive ? form.url : (targetTab?.form.url ?? form.url),
        headers: { ...userHeaders },
        body: stillActive ? form.body : (targetTab?.form.body ?? form.body),
      }
      const postResCtx: Record<string, any> = {
        status: info.status,
        statusText: info.statusText,
        headers: Object.fromEntries(info.headers.map(h => [h.key, h.value])),
        body: info.body,
        timeMs: info.timeMs,
        contentType: info.contentType,
        json: () => { try { return JSON.parse(info.body) } catch { return null } },
      }
      const postLogs = await runScript(postScriptText, postReqCtx, postResCtx)
      if (stillActive) {
        scriptLog.value.push(...postLogs)
      } else if (targetTab) {
        targetTab.scriptLog.push(...postLogs)
      }
    }

    // 将响应写入正确的页签
    if (stillActive) {
      if (scriptLog.value.length) respTab.value = 'scriptLog'
      response.value = info
      requestHeaders.value = sentHeaders
      if (!scriptLog.value.length) respTab.value = 'body'
    } else if (targetTab) {
      targetTab.response = info
      targetTab.requestHeaders = sentHeaders
      targetTab.error = ''
      targetTab.respTab = targetTab.scriptLog.length ? 'scriptLog' : 'body'
    }
    pushHistory(getFormGetters().form as any, info.status)
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e)
    if (activeTabId.value === sendingTabId) {
      error.value = msg
    } else {
      const targetTab = tabs.value.find(t => t.id === sendingTabId)
      if (targetTab) {
        targetTab.error = msg
        targetTab.response = null
      }
    }
    pushHistory(getFormGetters().form as any)
  } finally {
    if (activeTabId.value === sendingTabId) {
      loading.value = false
    } else {
      const targetTab = tabs.value.find(t => t.id === sendingTabId)
      if (targetTab) targetTab.loading = false
    }
  }
}

function copyResponseBody() {
  if (!response.value) return
  const text = respView.value === 'pretty' ? prettyBody.value : response.value.body
  navigator.clipboard.writeText(text).then(
    () => ElMessage.success('已复制响应内容'),
    () => ElMessage.error('复制失败'),
  )
}

// ====== 历史记录加载 ======
function loadHistory(h: HistoryItem) {
  form.method = h.method
  form.url = h.url
  form.params = h.params.length ? h.params : [emptyKV()]
  form.headers = h.headers.length ? h.headers : [emptyKV()]
  form.bodyType = h.bodyType
  form.body = h.body
  form.formBody = h.formBody.length ? h.formBody : [emptyKV()]
  form.preScript = h.preScript ?? ''
  form.postScript = h.postScript ?? ''
  form.categoryId = h.categoryId ?? undefined
  activeHistoryId.value = h.id
  currentRequestId.value = ''
  response.value = null
  error.value = ''
  scriptLog.value = []
  const tab = tabs.value.find(t => t.id === activeTabId.value)
  if (tab) tab.title = h.url ? `${h.method} ${shortenUrl(h.url)}` : ''
}

watch(() => form.method, (m) => {
  if (['GET', 'HEAD'].includes(m) && reqTab.value === 'body') reqTab.value = 'params'
})

// ====== 保存的请求 ======
async function saveCurrentRequest() {
  if (currentRequestId.value) {
    const existing = savedRequests.value.find(r => r.id === currentRequestId.value)
    if (existing) {
      const name = currentTabTitle.value.trim() || existing.name
      await updateRequest({
        ...existing, name,
        method: form.method, url: form.url,
        params: JSON.parse(JSON.stringify(form.params)),
        headers: JSON.parse(JSON.stringify(form.headers)),
        bodyType: form.bodyType, body: form.body,
        formBody: JSON.parse(JSON.stringify(form.formBody)),
        categoryId: form.categoryId ?? null,
        preScript: form.preScript, postScript: form.postScript,
      })
      currentTabTitle.value = name
      ElMessage.success('接口已更新')
      return
    }
  }
  try {
    let name = currentTabTitle.value.trim()
    if (!name || name === '新请求') {
      const { value } = await ElMessageBox.prompt('请输入接口名称', '保存接口', {
        confirmButtonText: '保存', cancelButtonText: '取消',
        inputPlaceholder: '例如：获取用户列表',
      })
      name = (value || '').trim()
      if (!name) return
    }
    const saved = await saveRequest({
      name, method: form.method, url: form.url,
      params: JSON.parse(JSON.stringify(form.params)),
      headers: JSON.parse(JSON.stringify(form.headers)),
      bodyType: form.bodyType, body: form.body,
      formBody: JSON.parse(JSON.stringify(form.formBody)),
      categoryId: form.categoryId ?? null,
      projectId: props.projectId ?? null,
      preScript: form.preScript, postScript: form.postScript,
    })
    currentRequestId.value = saved.id
    currentTabTitle.value = name
    ElMessage.success('接口已保存')
  } catch { /* cancelled */ }
}

async function handleSaveNewRequest(categoryId: string) {
  // categoryId 为选中分类（可能为空表示「全部接口」）；接口归属当前项目
  // 即使当前停留在系统说明页 / 空状态，也要切换到调试界面
  enterWorkspace()
  // 保存当前页签状态，然后新建一个空白页签
  saveCurrentTabStateWrapper()
  const newTab = createDefaultTabState()
  newTab.form.categoryId = categoryId || ''
  tabs.value.push(newTab)
  activeTabId.value = newTab.id
  currentRequestId.value = ''
  restoreTab(newTab)
}

function loadSavedRequest(req: SavedRequest) {
  enterWorkspace()
  if (currentRequestId.value !== req.id) {
    const existingTab = tabs.value.find(t => t.id !== activeTabId.value && t.currentRequestId === req.id)
    if (existingTab) { switchTab(existingTab.id); return }
    saveCurrentTabStateWrapper()
    const newTab = createDefaultTabState(req.name)
    newTab.currentRequestId = req.id
    newTab.form.method = req.method as Method
    newTab.form.url = req.url
    newTab.form.params = req.params?.length ? JSON.parse(JSON.stringify(req.params)) : [emptyKV()]
    newTab.form.headers = req.headers?.length ? JSON.parse(JSON.stringify(req.headers)) : [emptyKV()]
    newTab.form.bodyType = req.bodyType as BodyType
    newTab.form.body = req.body
    newTab.form.formBody = req.formBody?.length ? JSON.parse(JSON.stringify(req.formBody)) : [emptyKV()]
    newTab.form.preScript = req.preScript ?? ''
    newTab.form.postScript = req.postScript ?? ''
    newTab.form.categoryId = req.categoryId ?? undefined
    tabs.value.push(newTab)
    activeTabId.value = newTab.id
    restoreTab(newTab)
    return
  }
  form.method = req.method as Method
  form.url = req.url
  form.params = req.params?.length ? req.params : [emptyKV()]
  form.headers = req.headers?.length ? req.headers : [emptyKV()]
  form.bodyType = req.bodyType as BodyType
  form.body = req.body
  form.formBody = req.formBody?.length ? req.formBody : [emptyKV()]
  form.preScript = req.preScript ?? ''
  form.postScript = req.postScript ?? ''
  form.categoryId = req.categoryId ?? undefined
  currentRequestId.value = req.id
  activeHistoryId.value = ''
  response.value = null
  error.value = ''
  scriptLog.value = []
  const tab = tabs.value.find(t => t.id === activeTabId.value)
  if (tab) tab.title = req.name
}

async function handleDeleteSavedRequest(req: SavedRequest) {
  try {
    await ElMessageBox.confirm(
      `确定要删除接口「${req.name}」吗？`, '删除接口',
      { confirmButtonText: '删除', cancelButtonText: '取消', type: 'warning' },
    )
    await deleteRequest(req.id)
    tabs.value.forEach(t => { if (t.currentRequestId === req.id) t.currentRequestId = '' })
    if (currentRequestId.value === req.id) currentRequestId.value = ''
    ElMessage.success('已删除')
  } catch { /* cancelled */ }
}

// ====== 预览模式 ======
const enabledParams = computed(() => form.params.filter(p => p.enabled && p.key))
const enabledHeaders = computed(() => form.headers.filter(h => h.enabled && h.key))

const resolvedUrl = computed(() => {
  if (!form.url) return ''
  try {
    return resolveVariables(form.url)
  } catch { return form.url }
})

function resolvedHeaderValue(val: string): string {
  try { return resolveVariables(val) } catch { return val }
}

const curlCommand = computed(() => {
  if (!form.url) return '# 请先填写 URL'
  // L3：对所有动态片段做 shell 安全转义（单引号包覆 + 内部单引号转义）
  const shellQuote = (s: string) => `'${String(s).replace(/'/g, "'\\''")}'`
  const parts: string[] = ['curl']
  if (form.method !== 'GET') parts.push(`-X ${shellQuote(form.method)}`)

  let url = resolvedUrl.value
  const params = enabledParams.value
  if (params.length) {
    const qs = params.map(p => `${encodeURIComponent(resolvedHeaderValue(p.key))}=${encodeURIComponent(resolvedHeaderValue(p.value))}`).join('&')
    url += (url.includes('?') ? '&' : '?') + qs
  }
  parts.push(shellQuote(url))

  const headers = enabledHeaders.value
  for (const h of headers) {
    parts.push(`-H ${shellQuote(`${resolvedHeaderValue(h.key)}: ${resolvedHeaderValue(h.value)}`)}`)
  }

  if (form.bodyType !== 'none' && form.body.trim()) {
    const body = resolveVariables(form.body)
    parts.push(`-d ${shellQuote(body)}`)
  }

  return parts.join(' \\\n  ')
})

const previewBodyContent = computed(() => {
  if (!form.body.trim()) return ''
  const body = resolveVariables(form.body)
  if (form.bodyType === 'json') {
    try { return JSON.stringify(JSON.parse(body), null, 2) } catch { return body }
  }
  return body
})

// ====== 预览模式 - 详细信息 ======
const fullUrl = computed(() => {
  const base = resolvedUrl.value
  if (!base) return ''
  const params = enabledParams.value
  if (!params.length) return base
  const qs = params.map(p =>
    `${encodeURIComponent(resolvedHeaderValue(p.key))}=${encodeURIComponent(resolvedHeaderValue(p.value))}`
  ).join('&')
  return base + (base.includes('?') ? '&' : '?') + qs
})

const contentTypeLabel = computed(() => {
  switch (form.bodyType) {
    case 'json': return 'application/json'
    case 'form': return 'application/x-www-form-urlencoded'
    case 'text': return 'text/plain'
    default: return ''
  }
})

const bodyByteSize = computed(() => {
  if (form.bodyType === 'none' || !form.body.trim()) return 0
  const resolved = resolveVariables(form.body)
  return new Blob([resolved]).size
})

const enabledFormBody = computed(() => form.formBody.filter(f => f.enabled && f.key))

const resolvedRespHeaders = computed(() => {
  if (!response.value) return []
  return response.value.headers
})

function copyCurl() {
  navigator.clipboard.writeText(curlCommand.value).then(
    () => ElMessage.success('已复制 cURL 命令'),
    () => ElMessage.error('复制失败'),
  )
}

// ====== 设计模式（Mock 响应） ======
const mockResponses = reactive<MockResponse[]>([])
const mockSendFromDesign = ref(false)
const designDoc = ref('# 接口文档\n\n## 概述\n\n在此编写接口的详细说明文档，支持 **Markdown** 格式。\n\n## 请求参数\n\n| 参数 | 类型 | 必填 | 说明 |\n| --- | --- | --- | --- |\n| id | number | 是 | 资源 ID |\n\n## 响应示例\n\n```json\n{\n  "code": 0,\n  "data": {},\n  "message": "success"\n}\n```\n\n## 注意事项\n\n- 请确保请求头包含 `Authorization`\n- 分页参数从 `1` 开始\n')
const designDocTab = ref<'edit' | 'preview'>('preview')

const renderedDoc = computed(() => {
  try {
    // C2：Markdown 渲染结果经 DOMPurify 净化后注入，防止设计文档中的
    // <script>/<img onerror>/<iframe> 等造成 XSS（在 WebView 内可升级为 RCE）。
    const rawHtml = marked(designDoc.value, { async: false }) as string
    return DOMPurify.sanitize(rawHtml, {
      FORBID_TAGS: ['style', 'iframe', 'form', 'input', 'button', 'textarea', 'script', 'object', 'embed'],
      FORBID_ATTR: ['onerror', 'onload', 'onclick', 'onmouseover', 'style'],
    })
  } catch {
    return '<p>渲染失败</p>'
  }
})

function addMockResponse() {
  mockResponses.push({
    id: uid(),
    name: `场景 ${mockResponses.length + 1}`,
    statusCode: 200,
    statusText: 'OK',
    headers: '{\n  "Content-Type": "application/json"\n}',
    body: '{\n  "code": 0,\n  "data": {},\n  "message": "success"\n}',
    delay: 0,
    isActive: false,
  })
}

function removeMockResponse(idx: number) {
  mockResponses.splice(idx, 1)
}

function onMockToggle(idx: number) {
  const mock = mockResponses[idx]
  if (mock.isActive) {
    // 其他场景取消激活
    mockResponses.forEach((m, i) => { if (i !== idx) m.isActive = false })
  }
}

function onMockStatusChange(mock: MockResponse, code: number) {
  const opt = MOCK_STATUS_OPTIONS.find(o => o.value === code)
  if (opt) {
    mock.statusText = opt.label.split(' ').slice(1).join(' ')
  }
}

function formatMockBody(mock: MockResponse) {
  if (!mock.body.trim()) return
  try {
    mock.body = JSON.stringify(JSON.parse(mock.body), null, 2)
  } catch {
    ElMessage.warning('响应体不是合法 JSON')
  }
}

async function sendMockResponse(idx: number) {
  const mock = mockResponses[idx]
  if (!mock) return

  mockSendFromDesign.value = true
  loading.value = true
  error.value = ''

  // 模拟延迟
  if (mock.delay > 0) {
    await new Promise(r => setTimeout(r, mock.delay))
  }

  // 解析响应头
  let respHeaders: { key: string; value: string }[] = []
  let headerCount = 0
  try {
    const parsed = JSON.parse(mock.headers || '{}')
    respHeaders = Object.entries(parsed).map(([key, value]) => ({ key, value: String(value) }))
    headerCount = respHeaders.length
  } catch { /* ignore */ }

  const ct = respHeaders.find(h => h.key.toLowerCase() === 'content-type')?.value || 'application/json'
  const body = mock.body || ''

  response.value = {
    status: mock.statusCode,
    statusText: mock.statusText || 'OK',
    timeMs: mock.delay,
    size: new Blob([body]).size,
    headers: respHeaders,
    headerCount,
    body,
    contentType: ct,
  }
  loading.value = false
}

// ====== 快捷键支持 ======
function handleKeydown(e: KeyboardEvent) {
  // Ctrl+S / Cmd+S: 快速保存
  if ((e.ctrlKey || e.metaKey) && e.key === 's') {
    e.preventDefault()
    saveCurrentRequest()
  }
}

onMounted(async () => {
  loadPersistedHistory()
  window.addEventListener('keydown', handleKeydown)
  // 加载团队列表（认证/同步已在公共头部 AppLayout 初始化）
  await loadTeams()
  // 加载当前团队成员，恢复 canWrite 等权限；否则刷新直接进入此页时环境选择器等会被禁用
  await loadCurrentTeamMembers()
  // 刷新时从路由直接进入此页，必须主动加载项目列表，否则 projectName 因 projects 为空显示"未命名项目"
  await loadProjects()
  // 刷新直接进入本页时显式加载环境列表：环境 load 只挂在「环境管理」弹窗打开时，
  // 且 useEnvironments 内部对 selectedProjectId 的 watch 非 immediate——
  // 刷新时项目未变化，watch 不触发，会导致头部环境选择器为空、默认(上次激活)环境未选中。
  await loadEnvironments()
  // 刷新后若从本地恢复了上次打开的页签，将活跃页签内容同步到表单（含请求体）
  if (tabs.value.length && activeTabId.value) {
    const tab = tabs.value.find(t => t.id === activeTabId.value)
    if (tab) {
      showWelcome.value = false
      restoreTab(tab)
    } else {
      activeTabId.value = tabs.value[0].id
      restoreTab(tabs.value[0])
      showWelcome.value = false
    }
  }
})

onUnmounted(() => {
  window.removeEventListener('keydown', handleKeydown)
})
</script>

<style scoped>
@import './ApiDebuggerView.css';
</style>
