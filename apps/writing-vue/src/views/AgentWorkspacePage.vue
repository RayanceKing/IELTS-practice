<template>
  <section class="agent-page agent-workspace-page" data-agent-workspace>
    <header class="agent-page-header">
      <div class="agent-page-header__copy">
        <p class="agent-page-header__eyebrow">AI workspace</p>
        <h1>Agent 工作区</h1>
        <p class="agent-page-header__lede">把题目上下文、提示词和运行结果放在同一张工作台上。</p>
      </div>
      <div class="agent-page-header__status" :class="`is-${runState}`" role="status">
        <span class="agent-status-dot" aria-hidden="true"></span>
        <span>{{ runStateLabel }}</span>
      </div>
    </header>

    <div class="agent-workbench">
      <aside class="agent-panel agent-sidebar" aria-label="工作区文件">
        <div class="agent-panel__head agent-sidebar__head">
          <div>
            <p class="agent-panel__eyebrow">Workspace</p>
            <h2>本地工作区</h2>
          </div>
          <button
            class="agent-icon-button"
            type="button"
            aria-label="重置工作区预览"
            title="重置工作区预览"
            @click="resetWorkspace"
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <path d="M3 12a9 9 0 0 1 15.3-6.4L21 8"></path>
              <path d="M21 3v5h-5"></path>
              <path d="M21 12a9 9 0 0 1-15.3 6.4L3 16"></path>
              <path d="M3 21v-5h5"></path>
            </svg>
          </button>
        </div>

        <button class="agent-workspace-select" type="button" @click="toggleWorkspace">
          <span class="agent-workspace-select__icon" aria-hidden="true">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
              <path d="M3 7.5A2.5 2.5 0 0 1 5.5 5H10l2 2h6.5A2.5 2.5 0 0 1 21 9.5v7A2.5 2.5 0 0 1 18.5 19h-13A2.5 2.5 0 0 1 3 16.5v-9Z"></path>
            </svg>
          </span>
          <span class="agent-workspace-select__copy">
            <strong>{{ workspaceName }}</strong>
            <small>{{ workspaceStatus }}</small>
          </span>
          <svg class="agent-workspace-select__chevron" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <path d="m9 18 6-6-6-6"></path>
          </svg>
        </button>

        <div class="agent-file-tree">
          <div class="agent-file-tree__label">
            <span>文件</span>
            <span>{{ files.length }}</span>
          </div>
          <button
            v-for="file in files"
            :key="file.path"
            class="agent-file-row"
            :class="{ 'is-selected': selectedFile === file.path }"
            type="button"
            @click="selectFile(file.path)"
          >
            <span class="agent-file-row__icon" :class="`is-${file.kind}`" aria-hidden="true">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                <path d="M6 3h8l4 4v14H6z"></path>
                <path d="M14 3v5h5"></path>
                <path d="M9 13h6M9 17h6"></path>
              </svg>
            </span>
            <span class="agent-file-row__copy">
              <strong>{{ file.name }}</strong>
              <small>{{ file.meta }}</small>
            </span>
            <span v-if="selectedFile === file.path" class="agent-file-row__marker" aria-hidden="true"></span>
          </button>
        </div>

        <div class="agent-sidebar__footer">
          <span class="agent-sidebar__footer-dot" aria-hidden="true"></span>
          <span>本地预览模式</span>
        </div>
      </aside>

      <section class="agent-panel agent-prompt-panel" aria-label="提示词工作区">
        <div class="agent-panel__head">
          <div>
            <p class="agent-panel__eyebrow">Prompt</p>
            <h2>协作提示词</h2>
          </div>
          <span class="agent-model-badge">{{ modelLabel }}</span>
        </div>

        <div class="agent-prompt-toolbar" role="toolbar" aria-label="提示词工具">
          <div class="agent-segmented-control" role="tablist" aria-label="提示词模式">
            <button
              v-for="mode in promptModes"
              :key="mode.value"
              type="button"
              :class="{ 'is-active': promptMode === mode.value }"
              role="tab"
              :aria-selected="promptMode === mode.value"
              @click="promptMode = mode.value"
            >
              {{ mode.label }}
            </button>
          </div>
          <button class="agent-text-button" type="button" @click="resetPrompt">恢复示例</button>
        </div>

        <label class="agent-prompt-editor">
          <span class="sr-only">协作提示词</span>
          <textarea v-model="promptText" rows="12" spellcheck="false"></textarea>
          <span class="agent-prompt-editor__meta">{{ promptText.length }} characters</span>
        </label>

        <div class="agent-context-strip">
          <div class="agent-context-strip__label">
            <span class="agent-context-strip__icon" aria-hidden="true">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                <path d="M4 5.5A2.5 2.5 0 0 1 6.5 3H20v17H6.5A2.5 2.5 0 0 0 4 22V5.5Z"></path>
                <path d="M4 5.5V19"></path>
              </svg>
            </span>
            <span>上下文</span>
          </div>
          <button class="agent-context-chip" type="button" @click="selectFile(selectedFile)">
            <span>{{ selectedFileName }}</span>
            <span aria-hidden="true">×</span>
          </button>
          <button class="agent-add-context" type="button" aria-label="添加上下文" title="添加上下文" @click="selectNextFile">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <path d="M12 5v14M5 12h14"></path>
            </svg>
          </button>
        </div>

        <div class="agent-prompt-footer">
          <span class="agent-prompt-footer__hint">{{ promptMode === 'plan' ? '先整理步骤，再开始运行' : '准备好后运行本地预览' }}</span>
          <button class="agent-run-button" type="button" :disabled="runState === 'running'" @click="runPreview">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <path d="m8 5 11 7-11 7V5Z"></path>
            </svg>
            {{ runState === 'running' ? '运行中' : '运行预览' }}
          </button>
        </div>
      </section>

      <aside class="agent-panel agent-run-panel" aria-label="运行状态">
        <div class="agent-panel__head">
          <div>
            <p class="agent-panel__eyebrow">Run log</p>
            <h2>运行状态</h2>
          </div>
          <span class="agent-run-count">#{{ runCount }}</span>
        </div>

        <div class="agent-run-summary" :class="`is-${runState}`">
          <span class="agent-run-summary__icon" aria-hidden="true">
            <svg v-if="runState === 'complete'" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round"><path d="m5 12 4 4L19 6"></path></svg>
            <svg v-else-if="runState === 'running'" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3v4M12 17v4M3 12h4M17 12h4M5.6 5.6l2.8 2.8M15.6 15.6l2.8 2.8M18.4 5.6l-2.8 2.8M8.4 15.6l-2.8 2.8"></path></svg>
            <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round"><path d="M12 3v18M3 12h18"></path></svg>
          </span>
          <div>
            <strong>{{ runStateLabel }}</strong>
            <span>{{ runStateDetail }}</span>
          </div>
        </div>

        <ol class="agent-run-steps">
          <li v-for="step in runSteps" :key="step.key" :class="`is-${step.state}`">
            <span class="agent-run-step__index">{{ step.index }}</span>
            <span class="agent-run-step__copy">
              <strong>{{ step.label }}</strong>
              <small>{{ step.detail }}</small>
            </span>
            <span class="agent-run-step__state" aria-hidden="true"></span>
          </li>
        </ol>

        <div class="agent-output-panel">
          <div class="agent-output-panel__head">
            <span>输出</span>
            <span v-if="lastRunAt">{{ lastRunAt }}</span>
          </div>
          <p>{{ outputText }}</p>
        </div>
      </aside>
    </div>
  </section>
</template>

<script setup>
import { computed, onBeforeUnmount, ref } from 'vue'

const defaultPrompt = '请先阅读已选上下文，提炼关键事实，再给出一份简洁、可执行的学习建议。'
const promptText = ref(defaultPrompt)
const promptMode = ref('assist')
const selectedFile = ref('reading-notes.md')
const workspaceName = ref('IELTS Atlas Demo')
const runState = ref('idle')
const runCount = ref(0)
const lastRunAt = ref('')
const outputText = ref('运行结果会出现在这里。')
let runTimer = 0

const files = [
  { path: 'reading-notes.md', name: 'reading-notes.md', kind: 'markdown', meta: '12 KB · 已选上下文' },
  { path: 'writing-draft.txt', name: 'writing-draft.txt', kind: 'text', meta: '4 KB · 草稿' },
  { path: 'study-plan.json', name: 'study-plan.json', kind: 'json', meta: '2 KB · 计划' }
]

const promptModes = [
  { value: 'assist', label: '辅助' },
  { value: 'plan', label: '规划' }
]

const modelLabel = '本地配置模型'
const selectedFileName = computed(() => files.find((file) => file.path === selectedFile.value)?.name || files[0].name)
const workspaceStatus = computed(() => workspaceName.value === 'IELTS Atlas Demo' ? '演示工作区' : '本地工作区')
const runStateLabel = computed(() => ({ idle: '待命', running: '运行中', complete: '已完成' })[runState.value])
const runStateDetail = computed(() => {
  if (runState.value === 'running') return '正在准备本地预览'
  if (runState.value === 'complete') return '结果已更新，可继续编辑'
  return '等待提示词与上下文'
})
const runSteps = computed(() => {
  const state = runState.value
  return [
    { key: 'context', index: '01', label: '读取上下文', detail: selectedFileName.value, state: state === 'idle' ? 'pending' : 'complete' },
    { key: 'prompt', index: '02', label: '整理提示词', detail: promptMode.value === 'plan' ? '规划模式' : '辅助模式', state: state === 'running' ? 'active' : state === 'complete' ? 'complete' : 'pending' },
    { key: 'result', index: '03', label: '生成结果', detail: state === 'complete' ? '本地预览已完成' : '尚未运行', state: state === 'complete' ? 'complete' : 'pending' }
  ]
})

function selectFile(path) {
  selectedFile.value = path
}

function selectNextFile() {
  const currentIndex = files.findIndex((file) => file.path === selectedFile.value)
  selectedFile.value = files[(currentIndex + 1) % files.length].path
}

function toggleWorkspace() {
  workspaceName.value = workspaceName.value === 'IELTS Atlas Demo' ? 'My IELTS Workspace' : 'IELTS Atlas Demo'
}

function resetPrompt() {
  promptText.value = defaultPrompt
}

function resetWorkspace() {
  window.clearTimeout(runTimer)
  promptText.value = defaultPrompt
  promptMode.value = 'assist'
  selectedFile.value = files[0].path
  workspaceName.value = 'IELTS Atlas Demo'
  runState.value = 'idle'
  outputText.value = '运行结果会出现在这里。'
  lastRunAt.value = ''
}

function runPreview() {
  if (runState.value === 'running') return
  runState.value = 'running'
  runCount.value += 1
  outputText.value = '正在生成本地预览…'
  window.clearTimeout(runTimer)
  runTimer = window.setTimeout(() => {
    runState.value = 'complete'
    lastRunAt.value = new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
    outputText.value = `已基于 ${selectedFileName.value} 生成一份可继续编辑的学习建议。`
  }, 420)
}

onBeforeUnmount(() => window.clearTimeout(runTimer))
</script>
