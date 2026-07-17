<template>
  <div class="reading-page" :class="readingPageClassList" :style="readingPageStyle">
    <header class="reading-header header">
      <div class="header-content">
        <h1 id="exam-title">{{ pageTitle }}</h1>
        <p id="exam-subtitle">{{ headerSummary }}</p>
      </div>
      <div
        v-if="showSuiteReviewNav"
        id="review-nav-bar"
        class="review-nav-bar"
        :data-review-index="suiteReviewNavIndex"
        :data-review-total="suiteReviewNavTotal"
        data-view-mode="review"
      >
        <button
          type="button"
          data-review-dir="prev"
          :disabled="!canNavigateSuiteReviewPrev"
          @click="navigateSuiteReview('prev')"
        >
          上一篇
        </button>
        <button
          type="button"
          data-review-dir="next"
          :disabled="!canNavigateSuiteReviewNext"
          @click="navigateSuiteReview('next')"
        >
          下一篇
        </button>
      </div>
      <div class="reading-header-actions header-controls">
        <button
          v-if="payload"
          id="timer"
          class="reading-stat reading-timer"
          type="button"
          :class="{ paused: !timerRunning && !reviewMode }"
          :disabled="reviewMode"
          :title="timerRunning ? '暂停计时' : '继续计时'"
          data-reading-timer
          @click="toggleTimer"
        >
          {{ formattedTimer }}
        </button>
        <button
          v-if="payload"
          class="header-btn"
          id="settings-btn"
          title="阅读设置"
          aria-label="阅读设置"
          aria-controls="settings-panel"
          :aria-expanded="settingsPanelOpen"
          type="button"
          @click="toggleSettingsPanel"
        >
          ☰
        </button>
        <button
          v-if="payload"
          class="header-btn"
          id="note-btn"
          type="button"
          title="阅读笔记"
          aria-label="阅读笔记"
          aria-controls="notes-panel"
          :aria-expanded="notesPanelOpen"
          @click="toggleNotesPanel"
        >
          笔记
        </button>
      </div>
    </header>

    <div v-if="error" class="inline-message inline-message-error">
      <span>{{ error }}</span>
      <button class="btn-text" type="button" @click="loadAsset">重试</button>
    </div>

    <div v-if="loading" class="surface loading" role="status" aria-live="polite" aria-busy="true">正在加载阅读内容…</div>

    <div v-if="submitError" class="inline-message inline-message-error">
      <span>{{ submitError }}</span>
    </div>

    <div
      ref="settingsPanel"
      id="settings-panel"
      class="reading-floating-panel reading-settings-panel"
      v-show="settingsPanelOpen"
      role="dialog"
      aria-modal="true"
      aria-labelledby="settings-panel-title"
      @keydown="handleSettingsDialogKeydown"
    >
      <header class="settings-panel-header">
        <h2 id="settings-panel-title">阅读设置</h2>
        <button class="settings-panel-close" type="button" aria-label="关闭阅读设置" @click="closeFloatingPanels">关闭</button>
      </header>
      <div class="settings-section">
        <h3 class="settings-title">字号调整</h3>
        <div class="settings-options">
          <button
            v-for="option in fontSizeOptions"
            :key="option.value"
            class="settings-option"
            :class="{ active: readingFontSize === option.value }"
            type="button"
            :data-size="option.value"
            :style="option.style"
            @click="selectReadingFont(option.value)"
          >
            {{ option.label }}
          </button>
        </div>
      </div>
      <div class="settings-section" id="reading-coach-setting-section">
        <h3 class="settings-title">AI 教练</h3>
        <div class="settings-options">
          <button
            class="settings-option"
            type="button"
            data-reading-coach-enabled="true"
            :class="{ active: readingCoachEnabled === true }"
            :disabled="readingCoachSettingSaving"
            @click="updateReadingCoachEnabled(true)"
          >
            开启
          </button>
          <button
            class="settings-option"
            type="button"
            data-reading-coach-enabled="false"
            :class="{ active: readingCoachEnabled === false }"
            :disabled="readingCoachSettingSaving"
            @click="updateReadingCoachEnabled(false)"
          >
            关闭
          </button>
        </div>
        <p v-if="readingCoachSettingError" class="settings-help settings-help-error">
          {{ readingCoachSettingError }}
        </p>
      </div>
      <div
        v-if="activeSuiteSessionId"
        class="settings-section"
        id="suite-flow-mode-section"
      >
        <h3 class="settings-title">套题流程模式</h3>
        <div class="settings-options">
          <button
            class="settings-option"
            type="button"
            data-suite-flow-mode="auto"
            :class="{ active: suiteAutoAdvance === true }"
            @click="setSuiteAutoAdvance(true)"
          >
            自动跳转下一篇
          </button>
          <button
            class="settings-option"
            type="button"
            data-suite-flow-mode="manual"
            :class="{ active: suiteAutoAdvance === false }"
            @click="setSuiteAutoAdvance(false)"
          >
            提交后停留回看
          </button>
        </div>
      </div>
    </div>

    <div
      ref="notesPanel"
      id="notes-panel"
      class="reading-floating-panel reading-notes-panel"
      v-show="notesPanelOpen"
      role="dialog"
      aria-modal="true"
      aria-labelledby="notes-panel-title"
      @keydown="handleNotesDialogKeydown"
    >
      <header>
        <h3 id="notes-panel-title">阅读笔记</h3>
        <button id="close-note" type="button" @click="closeFloatingPanels">关闭</button>
      </header>
      <textarea ref="notesTextarea" v-model="notesText" aria-label="阅读笔记"></textarea>
      <p v-if="notesError" class="settings-help settings-help-error" role="alert">{{ notesError }}</p>
    </div>
    <div class="overlay" v-show="settingsPanelOpen || notesPanelOpen" @click="closeFloatingPanels"></div>

    <div
      id="selbar"
      class="reading-selection-toolbar"
      v-show="selectionToolbarVisible"
      :style="selectionToolbarStyle"
      @mousedown.prevent="keepSelectionToolbar = true"
      @mouseup="keepSelectionToolbar = false"
    >
      <button type="button" id="btnHL" data-role="highlight" @click="applySelectionHighlight('highlight')">高亮标记</button>
      <button type="button" id="btnUH" data-role="remove-highlight" @click="removeSelectionHighlight">取消高亮</button>
      <button type="button" id="btnNote" data-role="note" @click="applySelectionNote">添加笔记</button>
    </div>

    <div
      v-if="dictionaryBubble.visible"
      id="review-highlight-dictionary-bubble"
      class="review-highlight-dictionary-bubble"
      role="dialog"
      aria-live="polite"
      :style="{ left: `${dictionaryBubble.left}px`, top: `${dictionaryBubble.top}px` }"
    >
      <div class="vocab-bubble-head">
        <div>
          <h4 class="vocab-term">{{ dictionaryBubble.term }}</h4>
          <div v-if="dictionaryBubble.meta" class="vocab-meta">{{ dictionaryBubble.meta }}</div>
        </div>
        <button class="vocab-close" type="button" aria-label="关闭" @click="closeDictionaryBubble">×</button>
      </div>
      <template v-if="dictionaryBubble.parts.length">
        <div
          v-for="part in dictionaryBubble.parts"
          :key="part.term + part.meaning"
          class="vocab-part"
        >
          <div class="vocab-term vocab-part-term">{{ part.term }}</div>
          <div v-if="part.meta" class="vocab-meta">{{ part.meta }}</div>
          <div v-if="part.meaning" class="vocab-section">
            <div class="vocab-label">中文释义</div>
            <div class="vocab-text">{{ part.meaning }}</div>
          </div>
          <div v-if="part.definition" class="vocab-section">
            <div class="vocab-label">英文释义</div>
            <div class="vocab-text">{{ part.definition }}</div>
          </div>
        </div>
      </template>
      <div v-else class="vocab-section">
        <div class="vocab-label">{{ dictionaryBubble.found ? '释义' : '未收录' }}</div>
        <div class="vocab-text">{{ dictionaryBubble.meaning || '未找到该高亮内容。可先加入阅读高亮生词，后续在单词背诵中补充释义。' }}</div>
      </div>
      <div v-if="dictionaryBubble.definition" class="vocab-section">
        <div class="vocab-label">英文释义</div>
        <div class="vocab-text">{{ dictionaryBubble.definition }}</div>
      </div>
      <div v-if="dictionaryBubble.example" class="vocab-section">
        <div class="vocab-label">例句</div>
        <div class="vocab-text">{{ dictionaryBubble.example }}</div>
      </div>
      <div v-if="dictionaryBubble.sourceLine" class="vocab-section">
        <div class="vocab-label">来源</div>
        <div class="vocab-text">{{ dictionaryBubble.sourceLine }}</div>
      </div>
      <div class="vocab-actions">
        <button
          class="vocab-add"
          type="button"
          :disabled="dictionaryBubble.saved"
          @click="saveDictionaryBubbleWord"
        >
          {{ dictionaryBubble.saved ? '已加入' : '加入生词本' }}
        </button>
      </div>
    </div>

    <div v-if="isEndlessMode" class="inline-message endless-message" data-reading-endless-mode role="status" aria-live="polite">
      <span>{{ endlessStatusText }}</span>
      <div class="endless-actions">
        <button v-if="endlessNextAssetId" class="btn-text" type="button" :disabled="leaving" @click="goToNextEndlessAsset">下一篇</button>
        <button class="btn-text" type="button" :disabled="leaving" @click="stopEndlessMode">退出无尽模式</button>
      </div>
    </div>

    <p v-if="highlightRestoreWarning" class="settings-help settings-help-error" role="status">
      {{ highlightRestoreWarning }}
    </p>

    <p v-if="snapshotMessage" class="reading-action-status snapshot-message" role="status" aria-live="polite">
      {{ snapshotMessage }}
    </p>

    <p class="sr-only" aria-live="polite">{{ dragInteractionStatus }}</p>

    <section
      v-if="!loading && asset && payload"
      class="reading-workspace shell"
      :class="{ 'review-mode': reviewMode, 'memorize-mode': isMemorizeMode }"
      :style="readingWorkspaceStyle"
      data-practice-reading-page
      @click="handleWorkspaceClick"
      @keydown="handleWorkspaceKeydown"
      @dragstart="handleDragStart"
      @dragend="handleDragEnd"
      @dragover="handleDragOver"
      @dragleave="handleDragLeave"
      @drop="handleDrop"
    >
      <ReadingPassagePane
        ref="readingPassagePane"
        :passage-blocks="payload.passage.blocks"
        :official-passage-notes="officialPassageNotes"
      />

      <div
        id="reading-divider"
        :class="{ 'is-dragging': dividerDragging }"
        role="separator"
        aria-orientation="vertical"
        aria-label="调整原文和题目宽度"
        aria-valuemin="0"
        aria-valuemax="100"
        :aria-valuenow="Math.round(leftPanePercent)"
        tabindex="0"
        @pointerdown="startDividerDrag"
        @keydown="handleDividerKeydown"
      ></div>

      <ReadingQuestionPane
        :payload="payload"
        :is-memorize-mode="isMemorizeMode"
        :get-group-official-explanations="getGroupOfficialExplanations"
        :get-group-range="getGroupRange"
        :normalize-question-id="normalizeQuestionId"
        :get-display-label="getDisplayLabel"
        :format-review-answer="formatReviewAnswer"
        @question-input="handleQuestionInput"
      >
        <ReadingReviewPanel
          :submission="submission"
          :payload="payload"
          :analysis-signals="analysisSignals"
          :single-attempt-analysis="singleAttemptAnalysis"
          :single-attempt-analysis-llm="singleAttemptAnalysisLlm"
          :llm-review-status="llmReviewStatus"
          :llm-review-message="llmReviewMessage"
          :single-attempt-llm-diagnosis="singleAttemptLlmDiagnosis"
          :single-attempt-llm-actions="singleAttemptLlmActions"
          :single-attempt-llm-question-analyses="singleAttemptLlmQuestionAnalyses"
          :analysis-kind-rows="analysisKindRows"
          :format-duration="formatDuration"
          :format-density="formatDensity"
          :get-severity-label="getSeverityLabel"
          :get-question-kind-label="getQuestionKindLabel"
          :get-review-class="getReviewClass"
          :get-display-label="getDisplayLabel"
          :format-review-answer="formatReviewAnswer"
          :get-legacy-result-class="getLegacyResultClass"
          :get-review-label="getReviewLabel"
          @retry-review="runAutomaticReviewCoach"
        />
      </ReadingQuestionPane>
    </section>

    <ReadingCoachPanel
      v-if="readingCoachEnabled"
      :submission="submission"
      :reading-coach-open="readingCoachOpen"
      :coach-error="coachError"
      :coach-loading="coachLoading"
      :coach-status-text="coachStatusText"
      :selected-context="selectedContext"
      :coach-transcript="coachTranscript"
      :coach-response="coachResponse"
      :coach-quick-actions="coachQuickActions"
      :coach-selection-actions="coachSelectionActions"
      :coach-follow-ups="coachFollowUps"
      :coach-query="coachQuery"
      :can-ask-coach="canAskCoach"
      @toggle-panel="toggleReadingCoachPanel"
      @update:reading-coach-open="setReadingCoachOpen"
      @clear-selected-context="clearSelectedContext"
      @quick-action="runCoachQuickAction"
      @selection-action="runCoachSelectionAction"
      @follow-up="askCoachFollowUp"
      @update:coach-query="coachQuery = $event"
      @refresh-selected-context="refreshSelectedContext"
      @ask="askCoach"
    />

    <ReadingAnswerNav
      v-if="!loading && asset && payload"
      :asset="asset"
      :payload="payload"
      :suite-session="suiteSession"
      :answered-count="answeredCount"
      :return-route="returnRoute"
      :return-label="returnLabel"
      :reset-button-disabled="resetButtonDisabled"
      :reset-button-label="resetButtonLabel"
      :primary-button-disabled="primaryButtonDisabled"
      :primary-button-label="primaryButtonLabel"
      :loading="loading"
      :submitting="submitting"
      :leaving="leaving"
      :read-only-mode="readOnlyMode"
      :can-snapshot="canSnapshot"
      :has-answer="hasAnswer"
      :is-marked-question="isMarkedQuestion"
      :get-review-class="getReviewClass"
      :get-legacy-nav-status="getLegacyNavStatus"
      :is-active-question="isActiveQuestion"
      :get-display-label="getDisplayLabel"
      @scroll-to-question="scrollToQuestion"
      @toggle-marked-question="toggleMarkedQuestion"
      @leave="handleLeave"
      @reset="handleResetButton"
      @snapshot="snapshotAnswers"
      @primary="handlePrimaryButton"
    />

    <section v-if="!loading && !asset" class="surface empty-state">
      <strong>未找到阅读题目</strong>
      <span>返回练习库重新选择资源。</span>
    </section>
  </div>
</template>

<script setup>
import { computed, nextTick, onBeforeUnmount, onMounted, onUpdated, reactive, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { practiceReadingSuite, practiceSessions } from '@/api/practice-client.js'
import { readingCoachSettingsApi } from '@/modules/practice-reading/api'
import ReadingAnswerNav from '@/modules/practice-reading/components/ReadingAnswerNav.vue'
import ReadingCoachPanel from '@/modules/practice-reading/components/ReadingCoachPanel.vue'
import ReadingPassagePane from '@/modules/practice-reading/components/ReadingPassagePane.vue'
import ReadingQuestionPane from '@/modules/practice-reading/components/ReadingQuestionPane.vue'
import ReadingReviewPanel from '@/modules/practice-reading/components/ReadingReviewPanel.vue'
import { useReadingAsset } from '@/modules/practice-reading/useReadingAsset'
import { useReadingAnswers } from '@/modules/practice-reading/useReadingAnswers'
import { useReadingCoach } from '@/modules/practice-reading/useReadingCoach'
import { useReadingHighlights } from '@/modules/practice-reading/useReadingHighlights'
import { normalizeComparableText } from '@/modules/practice-reading/readingHighlightCore.js'
import { useReadingModeFlow } from '@/modules/practice-reading/useReadingModeFlow'
import { canSnapshotReadingAnswers } from '@/modules/practice-reading/readingModeFlowCore.js'
import { useReadingTimer } from '@/modules/practice-reading/useReadingTimer'
import { useReadingAttempt } from '@/modules/practice-reading/useReadingAttempt'
import {
  readingFontSizeOptions as fontSizeOptions,
  useReadingUiPreferences
} from '@/modules/practice-reading/useReadingUiPreferences'
import { useReadingInteractions } from '@/modules/practice-reading/useReadingInteractions'
import {
  escapeCss,
  expandQuestionSequence,
  normalizeQuestionId,
  resolveAnswerAliases as resolveAnswerAliasesFromIds
} from '@/modules/practice-reading/readingQuestionIds'
import { isTauriRuntime } from '@/api/tauri-bridge.js'
import { getOpenReadingDraft } from '@/api/reading-repository.js'
import {
  saveEndlessPassageDraft,
  saveSuitePassageDraft
} from '@/api/modes-repository.js'

const EXPLANATION_SPLIT_KINDS = new Set([
  'single_choice',
  'multi_choice',
  'true_false_not_given',
  'yes_no_not_given'
])
const coachQuickActions = [
  { id: 'hint', label: '给我提示' },
  { id: 'explain', label: '解释这题' },
  { id: 'review', label: '复盘错题' },
  { id: 'similar', label: '推荐同类题' }
]
const coachSelectionActions = [
  { id: 'explain_selection', label: '解释选中' },
  { id: 'locate_evidence', label: '定位证据' },
  { id: 'find_paraphrases', label: '同义替换' }
]

const props = defineProps({
  assetId: {
    type: String,
    required: true
  },
  sessionId: {
    type: String,
    default: ''
  },
  suiteSessionId: {
    type: String,
    default: ''
  }
})

const route = useRoute()
const router = useRouter()
const {
  asset,
  payload,
  loading,
  error,
  loadReadingAsset,
  clearReadingAssetError
} = useReadingAsset()
const submitting = ref(false)
const leaving = ref(false)
const submitError = ref('')
const snapshotMessage = ref('')
const submission = ref(null)
const suiteSession = ref(null)
const answerTimeline = reactive({})
const markedQuestions = ref([])
const interactionCount = ref(0)
const endlessCountdown = ref(0)
const endlessNextAssetId = ref('')
const readingCoachEnabled = ref(true)
const readingCoachSettingSaving = ref(false)
const readingCoachSettingError = ref('')
const leftPanePercent = ref(50)
const dividerDragging = ref(false)
let dividerPointerId = null
const activeQuestionVisit = {
  questionId: '',
  startedAtMs: 0
}
const activeQuestionId = ref('')
const readingPassagePane = ref(null)
const readingAttempt = useReadingAttempt()
let tauriAttemptId = ''
let draftAutosaveTimer = null
let snapshotMessageTimer = null

const {
  settingsPanelOpen,
  notesPanelOpen,
  settingsPanel,
  notesPanel,
  notesTextarea,
  notesText,
  notesError,
  readingFontSize,
  suiteAutoAdvance,
  readingPageClassList: preferencePageClassList,
  readingPageStyle,
  initializeReadingPreferences,
  toggleSettingsPanel,
  toggleNotesPanel,
  handleSettingsDialogKeydown,
  handleNotesDialogKeydown,
  closeFloatingPanels,
  selectReadingFont,
  setSuiteAutoAdvance,
  loadReadingNotes,
  clearReadingNotesDraft
} = useReadingUiPreferences({
  assetSource: () => asset.value
})

const activeSuiteSessionId = computed(() => {
  const fromProp = String(props.suiteSessionId || '').trim()
  const fromQuery = Array.isArray(route.query.suiteSessionId)
    ? route.query.suiteSessionId[0]
    : route.query.suiteSessionId
  return fromProp || String(fromQuery || '').trim()
})
const pageTitle = computed(() => payload.value?.meta?.title || asset.value?.title || '阅读练习')
function readRouteQueryValue(key) {
  const value = route.query?.[key]
  return Array.isArray(value) ? value[0] : value
}
const activeEndlessSessionId = computed(() => {
  return String(readRouteQueryValue('endlessSessionId') || '').trim()
})

function normalizePracticeModeQueryValue(value) {
  return String(value || '').trim().toLowerCase()
}

function shouldNormalizeLegacyMemorizeQuery() {
  return !String(props.sessionId || route.params.sessionId || '').trim()
    && !activeSuiteSessionId.value
    && normalizePracticeModeQueryValue(readRouteQueryValue('mode')) === 'review'
    && !normalizePracticeModeQueryValue(readRouteQueryValue('practiceMode'))
}

function normalizeLegacyMemorizeQuery() {
  if (!shouldNormalizeLegacyMemorizeQuery()) return false
  router.replace({
    name: route.name || 'PracticeReading',
    params: route.params,
    query: {
      ...route.query,
      mode: 'memorize',
      practiceMode: 'memorize'
    }
  })
  return true
}

const isEndlessMode = computed(() => {
  const mode = normalizePracticeModeQueryValue(readRouteQueryValue('mode'))
  return mode === 'endless' && !activeSuiteSessionId.value
})
const isMemorizeMode = computed(() => {
  const mode = normalizePracticeModeQueryValue(readRouteQueryValue('mode'))
  const practiceMode = normalizePracticeModeQueryValue(readRouteQueryValue('practiceMode'))
  return (mode === 'memorize' || practiceMode === 'memorize') && !props.sessionId
})
const activeMemorizeAttemptId = ref(String(readRouteQueryValue('memorizeAttemptId') || '').trim())
const headerSummary = computed(() => {
  if (!payload.value) return '正在准备阅读练习…'
  const category = payload.value.meta?.category || asset.value?.category || '阅读'
  const questionCount = Number(payload.value.questionCount ?? payload.value.questionOrder?.length) || 0
  const mode = activeSuiteSessionId.value
    ? '套题练习'
    : (isEndlessMode.value ? '无尽练习' : (isMemorizeMode.value ? '背题模式' : '单篇练习'))
  return `${category} · ${questionCount} 题 · ${mode}`
})
const returnRoute = computed(() => (
  activeSuiteSessionId.value
    ? { name: 'PracticeReadingSuite', params: { sessionId: activeSuiteSessionId.value } }
    : { name: 'PracticeLibrary' }
))
const returnLabel = computed(() => (activeSuiteSessionId.value ? '返回套题进度' : '返回练习库'))
const reviewMode = computed(() => Boolean(submission.value))
const readOnlyMode = computed(() => reviewMode.value || isMemorizeMode.value)
const {
  answeredCount,
  initializeAnswers: initializeReadingAnswers,
  assignAnswer,
  setAnswer,
  toggleAnswerOption,
  getAnswerValue,
  getRawAnswer,
  getAnswerEntries,
  hasAnswer,
  isOptionSelected,
  snapshotAnswers: snapshotAnswerMap,
  getAnswerFingerprint
} = useReadingAnswers({
  payloadSource: () => payload.value,
  readOnlySource: readOnlyMode,
  onTrack: recordAnswerTimeline,
  onSyncNative: syncNativeControl,
  onMutate: () => {
    clearSnapshotMessage()
    submitError.value = ''
  }
})
const {
  currentDragPayload,
  getInteraction,
  getDisplayLabel,
  isChoiceControl,
  isDragDropControl,
  getOptions,
  isMultiValueCheckbox,
  getDragDropGroup,
  getDragDropGroupQuestionIds,
  allowsDragOptionReuse,
  getSelectedOption,
  getSelectedOptionLabel,
  findQuestionUsingDragOption,
  isDragOptionUnavailable,
  setDragDropAnswer,
  clearDragDropAnswer,
  dropOnAnswerSlot,
  dragInteractionStatus,
  handleWorkspaceClick,
  handleWorkspaceKeydown,
  handleDragStart,
  handleDragEnd,
  handleDragOver,
  handleDragLeave,
  handleDrop,
  findNativeDropzonesByQuestionId,
  ensureDropzoneHolder,
  syncDropzoneControl,
  setReadOnlyDomControls,
  clearDragHoverState,
  getNativeDropzoneElement,
  getDragPoolElement,
  resolveDropzoneQuestionId
} = useReadingInteractions({
  payloadSource: () => payload.value,
  readOnlyModeSource: readOnlyMode,
  getAnswerValue,
  getRawAnswer,
  assignAnswer,
  recordQuestionVisit: (questionId) => recordQuestionVisit(questionId)
})
const {
  elapsedSeconds,
  timerRunning,
  suiteTimerState,
  formattedTimer,
  applySuiteTimerState,
  applyPracticeTimerState,
  getPracticeTimerSnapshot,
  resolvePracticeTiming,
  startPracticeTimer,
  stopPracticeTimer,
  toggleTimer: togglePracticeTimer,
  resetPracticeTimerClock,
  setPracticeTimerElapsedSeconds
} = useReadingTimer({
  activeSuiteSessionId,
  reviewMode,
  suiteTimerSource: () => suiteSession.value?.timer,
  onAutoSubmit: () => submitAnswers()
})
const {
  coachQuery,
  coachLoading,
  coachError,
  coachResponse,
  selectedContext,
  readingCoachOpen,
  llmReviewStatus,
  llmReviewMessage,
  canAskCoach,
  coachStatusText,
  coachTranscript,
  coachFollowUps,
  resetReadingCoachState,
  setReadingCoachOpen,
  hydrateReadingCoachFromSubmission,
  toggleReadingCoachPanel,
  refreshSelectedContext,
  clearSelectedContext,
  askCoach,
  askCoachFollowUp,
  runCoachQuickAction,
  runCoachSelectionAction,
  runAutomaticReviewCoach,
  queueAutomaticReviewRefresh
} = useReadingCoach({
  submissionSource: () => submission.value,
  setSubmission: (nextSubmission) => {
    submission.value = nextSubmission
  },
  assetIdSource: () => submission.value?.examId || asset.value?.id || props.assetId,
  resolveCoachMode: () => {
    if (activeSuiteSessionId.value) return 'suite'
    if (isEndlessMode.value) return 'endless'
    return props.sessionId || route.params.sessionId ? 'review' : 'single'
  },
  readCoachEnabled: () => readingCoachEnabled.value,
  readSelectedContext,
  getDisplayLabel,
  formatReviewAnswer,
  flushActiveQuestionVisit,
  snapshotSubmission,
  onSubmissionHydrated: (loadedSubmission) => {
    restoreSubmittedMetadata(loadedSubmission)
    if (loadedSubmission?.answers) {
      Object.entries(loadedSubmission.answers).forEach(([questionId, value]) => {
        assignAnswer(questionId, value)
      })
    }
    syncDomAnswers()
  }
})
const {
  selectionToolbarVisible,
  selectionToolbarStyle,
  keepSelectionToolbar,
  highlightSnapshot,
  highlightRestoreWarning,
  dictionaryBubble,
  normalizeHighlightSnapshot: normalizeHighlightSnapshotState,
  resetHighlightUiState: resetHighlightUiStateFromComposable,
  snapshotHighlights,
  restoreHighlightsFromRecords,
  applySelectionHighlight,
  applySelectionNote,
  removeSelectionHighlight,
  saveDictionaryBubbleWord,
  closeDictionaryBubble,
  attachHighlightDocumentListeners,
  detachHighlightDocumentListeners,
  hydrateHighlightsFromStore,
  getTextNodes
} = useReadingHighlights({
  assetIdSource: () => asset.value?.id || props.assetId,
  reviewModeSource: () => reviewMode.value,
  getAttemptId: () => tauriAttemptId,
  ensureAttemptId: () => {
    if (!tauriAttemptId) {
      tauriAttemptId = readingAttempt.newAttemptId()
    }
    return tauriAttemptId
  },
  onAttemptEnsured: () => {
    void persistTauriDraft()
  },
  onInteraction: () => recordInteraction(),
  notesText,
  toggleNotesPanel
})
const canSubmit = computed(() => Boolean(
  asset.value
  && payload.value
  && !loading.value
  && !submitting.value
  && !leaving.value
  && !readOnlyMode.value
))
const canSnapshot = computed(() => canSnapshotReadingAnswers({
  isTauriRuntime: isTauriRuntime(),
  hasAsset: Boolean(asset.value?.id),
  hasPayload: Boolean(payload.value),
  loading: loading.value,
  submitting: submitting.value,
  leaving: leaving.value,
  readOnly: readOnlyMode.value,
  isEndlessMode: isEndlessMode.value
}))
const canRecycleSubmittedAttempt = computed(() => Boolean(
  reviewMode.value
  && !activeSuiteSessionId.value
  && !isEndlessMode.value
  && !String(props.sessionId || route.params.sessionId || '').trim()
  && !submitting.value
))
const readingPageClassList = computed(() => ({
  ...preferencePageClassList.value,
  'reading-memorize-mode': isMemorizeMode.value,
  'reading-pane-resizing': dividerDragging.value
}))
const readingWorkspaceStyle = computed(() => ({
  '--atlas-reading-left-pane-width': `${leftPanePercent.value}%`
}))
const primaryButtonLabel = computed(() => {
  if (submitting.value) return '提交中…'
  if (isMemorizeMode.value) return '退出背题'
  if (reviewMode.value) return '已提交'
  return '提交作答'
})
const primaryButtonDisabled = computed(() => {
  if (isMemorizeMode.value) return leaving.value
  return !canSubmit.value
})
const resetButtonLabel = computed(() => {
  if (isMemorizeMode.value) return '转为练习'
  if (reviewMode.value) return '重新作答'
  return '清空作答'
})
const resetButtonDisabled = computed(() => {
  if (isMemorizeMode.value) return leaving.value
  if (reviewMode.value) return !canRecycleSubmittedAttempt.value || leaving.value
  return submitting.value || leaving.value
})
const endlessStatusText = computed(() => {
  if (endlessCountdown.value > 0 && endlessNextAssetId.value) {
    return `无尽模式：${endlessCountdown.value} 秒后进入下一篇`
  }
  return '无尽模式进行中，提交后会自动进入下一篇。'
})
const officialReviewExplanations = computed(() => (
  reviewMode.value && payload.value?.reviewExplanations
    ? payload.value.reviewExplanations
    : null
))
const officialPassageNotes = computed(() => {
  const notes = officialReviewExplanations.value?.passageNotes
  return Array.isArray(notes)
    ? notes
        .map((note, index) => ({
          label: String(note?.label || `第 ${index + 1} 段`).trim(),
          text: String(note?.text || '').trim()
        }))
        .filter((note) => note.text)
    : []
})
const officialQuestionExplanationSections = computed(() => {
  const sections = officialReviewExplanations.value?.questionExplanations
  return Array.isArray(sections)
    ? sections
        .map((section, index) => normalizeOfficialQuestionExplanationSection(section, index))
        .filter(Boolean)
    : []
})
const suiteSequence = computed(() => (
  Array.isArray(suiteSession.value?.sequence) ? suiteSession.value.sequence : []
))
const currentSuitePassageIndex = computed(() => {
  const currentAssetId = String(asset.value?.id || props.assetId || route.params.assetId || '').trim()
  if (!currentAssetId) return -1
  return suiteSequence.value.findIndex((entry) => (
    String(entry?.assetId || '').trim() === currentAssetId
    || String(entry?.examId || '').trim() === currentAssetId
  ))
})
const suiteReviewNavIndex = computed(() => Math.max(0, currentSuitePassageIndex.value))
const suiteReviewNavTotal = computed(() => suiteSequence.value.length || 0)
const showSuiteReviewNav = computed(() => Boolean(
  reviewMode.value
  && activeSuiteSessionId.value
  && suiteSequence.value.length > 1
  && currentSuitePassageIndex.value >= 0
))
const canNavigateSuiteReviewPrev = computed(() => findSuiteReviewNavigationTarget('prev') !== null)
const canNavigateSuiteReviewNext = computed(() => findSuiteReviewNavigationTarget('next') !== null)
const analysisSignals = computed(() => submission.value?.analysisSignals || submission.value?.analysisArtifacts?.analysisSignals || null)
const singleAttemptAnalysis = computed(() => submission.value?.singleAttemptAnalysis || submission.value?.analysisArtifacts?.singleAttemptAnalysis || null)
const singleAttemptAnalysisLlm = computed(() => (
  submission.value?.singleAttemptAnalysisLlm
  || submission.value?.analysisArtifacts?.singleAttemptAnalysisLlm
  || null
))
const singleAttemptLlmDiagnosis = computed(() => (
  Array.isArray(singleAttemptAnalysisLlm.value?.diagnosis)
    ? singleAttemptAnalysisLlm.value.diagnosis
        .map((entry, index) => ({
          code: String(entry?.code || entry?.type || `coach_diag_${index + 1}`),
          reason: String(entry?.reason || entry?.message || '').trim()
        }))
        .filter((entry) => entry.reason)
    : []
))
const singleAttemptLlmActions = computed(() => (
  Array.isArray(singleAttemptAnalysisLlm.value?.nextActions)
    ? singleAttemptAnalysisLlm.value.nextActions
        .map((entry, index) => ({
          type: String(entry?.type || `coach_action_${index + 1}`),
          target: String(entry?.target || 'reading').trim() || 'reading',
          instruction: String(entry?.instruction || entry?.action || '').trim()
        }))
        .filter((entry) => entry.instruction)
    : []
))
const singleAttemptLlmQuestionAnalyses = computed(() => (
  Array.isArray(singleAttemptAnalysisLlm.value?.reviewQuestionAnalyses)
    ? singleAttemptAnalysisLlm.value.reviewQuestionAnalyses
        .map((entry, index) => {
          const rawQuestionNumber = String(entry?.questionNumber || entry?.questionId || '').replace(/^q/i, '').trim()
          const questionLabel = `第 ${rawQuestionNumber || index + 1} 题`
          return {
            questionLabel,
            likelyMistake: String(entry?.likelyMistake || '').trim(),
            whyUserChoseWrong: String(entry?.whyUserChoseWrong || '').trim(),
            whyCorrectAnswerWorks: String(entry?.whyCorrectAnswerWorks || '').trim(),
            whyWrongAnswerFails: String(entry?.whyWrongAnswerFails || '').trim(),
            nextRule: String(entry?.nextRule || '').trim()
          }
        })
        .filter((entry) => (
          entry.likelyMistake
          || entry.whyUserChoseWrong
          || entry.whyCorrectAnswerWorks
          || entry.whyWrongAnswerFails
          || entry.nextRule
        ))
    : []
))
const analysisKindRows = computed(() => {
  const rows = singleAttemptAnalysis.value?.radar?.byQuestionKind
  if (Array.isArray(rows) && rows.length) {
    return rows
  }
  const fallback = submission.value?.questionTypePerformance || {}
  return Object.values(fallback)
})

function clearSnapshotMessage() {
  snapshotMessage.value = ''
  if (snapshotMessageTimer) {
    clearTimeout(snapshotMessageTimer)
    snapshotMessageTimer = null
  }
}

function showSnapshotMessage(message, durationMs = 4200) {
  clearSnapshotMessage()
  snapshotMessage.value = message
  snapshotMessageTimer = setTimeout(() => {
    snapshotMessage.value = ''
    snapshotMessageTimer = null
  }, durationMs)
}

onMounted(async () => {
  await initializeReadingPreferences()
  attachHighlightDocumentListeners()
  await loadReadingCoachPreference()
  await loadAsset()
})

onBeforeUnmount(() => {
  flushActiveQuestionVisit()
  stopPracticeTimer()
  if (!reviewMode.value && !isMemorizeMode.value && !isEndlessMode.value) {
    void persistTauriDraft().catch((error) => {
      console.warn('阅读页面卸载时保存草稿失败', error)
    })
  }
  clearEndlessTimer()
  if (isMemorizeMode.value) void finishActiveMemorizeSession()
  if (draftAutosaveTimer) {
    clearTimeout(draftAutosaveTimer)
    draftAutosaveTimer = null
  }
  clearSnapshotMessage()
  detachHighlightDocumentListeners()
  removeDividerDragListeners()
})

onUpdated(() => {
  if (!asset.value?.id || !payload.value?.questionOrder?.length) {
    return
  }
  syncDomAnswers()
  if (readOnlyMode.value) {
    setReadOnlyDomControls(true)
  }
})

watch(() => props.assetId, () => {
  loadAsset()
})

watch(() => props.sessionId, () => {
  loadAsset()
})

watch(() => activeSuiteSessionId.value, () => {
  loadAsset()
})

watch(() => [readRouteQueryValue('mode'), readRouteQueryValue('practiceMode')], () => {
  loadAsset()
})

watch(
  () => ({
    assetId: asset.value?.id || '',
    questionCount: Array.isArray(payload.value?.questionOrder) ? payload.value.questionOrder.length : 0,
    answers: JSON.stringify(snapshotAnswerMap()),
    readOnly: readOnlyMode.value
  }),
  async ({ assetId, questionCount }) => {
    if (!assetId || !questionCount) {
      return
    }
    await nextTick()
    syncDomAnswers()
    if (readOnlyMode.value) {
      setReadOnlyDomControls(true)
    }
  },
  { flush: 'post' }
)

async function loadAsset() {
  const normalizedAssetId = String(props.assetId || route.params.assetId || '').trim()
  const replaySessionId = String(props.sessionId || route.params.sessionId || '').trim()
  if (!normalizedAssetId) {
    error.value = '缺少阅读资源编号'
    return
  }
  if (normalizeLegacyMemorizeQuery()) {
    return
  }

  clearReadingAssetError()
  submitError.value = ''
  clearSnapshotMessage()
  submission.value = null
  suiteSession.value = null
  resetAttemptMetadata()
  resetReadingCoachState()
  closeFloatingPanels()
  resetHighlightUiStateFromComposable()
  clearReadingNotesDraft()
  clearEndlessTimer()
  resetPracticeTimerClock()
  endlessNextAssetId.value = ''
  try {
    const data = await loadReadingAsset(normalizedAssetId)
    if (isEndlessMode.value && !(await reconcileEndlessRoute())) {
      return
    }
    initializeReadingAnswers(data?.payload, { prefillAnswerKey: isMemorizeMode.value })
    loadReadingNotes()
    if (activeSuiteSessionId.value) {
      try {
        suiteSession.value = await practiceReadingSuite.get(activeSuiteSessionId.value)
        applySuiteTimerState()
      } catch (suiteLoadError) {
        if (!replaySessionId) {
          throw suiteLoadError
        }
        console.warn('加载套题进度失败，继续回放已提交阅读记录:', suiteLoadError)
        suiteSession.value = null
      }
    }
    if (replaySessionId) {
      await loadSubmittedSession(replaySessionId)
    } else if (isTauriRuntime() && !isMemorizeMode.value) {
      await hydrateOpenDraft(
        normalizedAssetId,
        activeSuiteSessionId.value || null,
        activeEndlessSessionId.value || null
      )
    }
  } catch (loadError) {
    console.error('加载阅读资源失败:', loadError)
  }
  if (asset.value && payload.value) {
    await ensureMemorizeSession()
    await nextTick()
    syncDomAnswers()
    if (isTauriRuntime()) {
      const attemptScope = tauriAttemptId || replaySessionId || null
      const passageDocument = String(readingPassagePane.value?.$el?.textContent || '').trim()
      await hydrateHighlightsFromStore(asset.value.id, attemptScope, passageDocument || null)
    }
    restoreHighlightsFromRecords(highlightSnapshot.value)
    applyMemorizeStudyLayer()
    if (readOnlyMode.value) {
      setPracticeTimerElapsedSeconds(Math.max(0, Number(submission.value?.duration || 0)))
      setReadOnlyDomControls(true)
    } else {
      startPracticeTimer()
    }
  }
}

async function loadSubmittedSession(sessionId) {
  const state = await practiceSessions.getState('reading', sessionId)
  const loadedSubmission = state?.submission || null
  if (!loadedSubmission) {
    throw new Error('未找到可回放的阅读提交记录')
  }
  const expectedAssetId = String(asset.value?.id || '').trim()
  const actualAssetId = String(loadedSubmission.assetId || loadedSubmission.examId || '').trim()
  if (expectedAssetId && actualAssetId && expectedAssetId !== actualAssetId) {
    throw new Error('阅读回放记录与当前题目不匹配')
  }
  submission.value = loadedSubmission
  // Field contraction: store may omit correctAnswer; fill from asset answerKey for review UI.
  fillCorrectAnswersFromPayload(loadedSubmission)
  const attemptScope = String(loadedSubmission.sessionId || loadedSubmission.attemptId || sessionId || '').trim()
  if (isTauriRuntime() && asset.value?.id) {
    const persisted = await hydrateHighlightsFromStore(asset.value.id, attemptScope || null)
    if (!persisted.length) {
      highlightSnapshot.value = normalizeHighlightSnapshotState(
        loadedSubmission.highlights || loadedSubmission.analysisArtifacts?.highlights || []
      )
    }
  } else {
    highlightSnapshot.value = normalizeHighlightSnapshotState(
      loadedSubmission.highlights || loadedSubmission.analysisArtifacts?.highlights || []
    )
  }
  hydrateReadingCoachFromSubmission(loadedSubmission, {
    open: true,
    pendingIfMissing: true,
    successMessage: 'AI 复盘已载入',
    pendingMessage: 'AI 复盘待补全'
  })
  restoreSubmittedMetadata(loadedSubmission)
  if (loadedSubmission.answers) {
    Object.entries(loadedSubmission.answers).forEach(([questionId, value]) => {
      assignAnswer(questionId, value)
    })
  }
  await nextTick()
  syncDomAnswers()
  setReadOnlyDomControls(true)
  restoreHighlightsFromRecords(highlightSnapshot.value)
  restoreSubmittedViewport(loadedSubmission)
  if (readingCoachEnabled.value) {
    if (!loadedSubmission.singleAttemptAnalysisLlm && !loadedSubmission.analysisArtifacts?.singleAttemptAnalysisLlm) {
      queueAutomaticReviewRefresh(loadedSubmission.sessionId)
    }
  }
}

async function hydrateOpenDraft(assetId, suiteId = null, endlessSessionId = null) {
  try {
    const expectedSuiteId = String(suiteId || '').trim()
    const { attempt, timer } = await getOpenReadingDraft(
      assetId,
      expectedSuiteId || null,
      String(endlessSessionId || '').trim() || null
    )
    if (!attempt?.id) return
    const draftSuiteId = String(attempt.suiteId || attempt.suite_id || '').trim()
    if (draftSuiteId !== expectedSuiteId) {
      console.warn('忽略不属于当前阅读范围的草稿', {
        attemptId: attempt.id,
        expectedSuiteId,
        draftSuiteId
      })
      return
    }
    tauriAttemptId = String(attempt.id)
    if (timer) applyPracticeTimerState(timer)
    const answers = {}
    const marked = []
    for (const entry of attempt.answers || []) {
      const questionId = String(entry.questionId || entry.question_id || '').trim()
      if (!questionId) continue
      if (entry.answer != null) answers[questionId] = entry.answer
      if (entry.marked) marked.push(questionId)
      answerTimeline[questionId] = {
        firstAnsweredAt: entry.answeredAt || entry.answered_at || null,
        lastAnsweredAt: entry.answeredAt || entry.answered_at || null,
        changeCount: Math.max(0, Number(entry.changeCount ?? entry.change_count) || 0),
        visitCount: Math.max(0, Number(entry.visitCount ?? entry.visit_count) || 0),
        elapsedMs: Math.max(0, Number(entry.elapsedMs ?? entry.elapsed_ms) || 0),
        lastFingerprint: getAnswerFingerprint(entry.answer)
      }
    }
    Object.entries(answers).forEach(([questionId, value]) => {
      assignAnswer(questionId, value)
    })
    if (marked.length) {
      markedQuestions.value = marked
    }
    showSnapshotMessage('已恢复未提交草稿。')
  } catch (draftError) {
    console.warn('加载阅读草稿失败:', draftError)
  }
}

function fillCorrectAnswersFromPayload(loadedSubmission) {
  const comparison = loadedSubmission?.answerComparison
  const answerKey = payload.value?.answerKey || payload.value?.answers || null
  if (!comparison || typeof comparison !== 'object' || !answerKey || typeof answerKey !== 'object') {
    return
  }
  Object.keys(comparison).forEach((questionId) => {
    const row = comparison[questionId]
    if (!row || row.correctAnswer != null) return
    if (Object.prototype.hasOwnProperty.call(answerKey, questionId)) {
      row.correctAnswer = answerKey[questionId]
    }
  })
}

function restoreSubmittedViewport(loadedSubmission) {
  const scrollY = Number(loadedSubmission?.metadata?.scrollY ?? loadedSubmission?.scrollY)
  if (!Number.isFinite(scrollY) || scrollY <= 0 || typeof window === 'undefined') {
    return
  }
  window.requestAnimationFrame(() => {
    window.scrollTo(0, Math.max(0, Math.round(scrollY)))
  })
}

function resetAttemptMetadata() {
  flushActiveQuestionVisit()
  activeQuestionVisit.questionId = ''
  activeQuestionVisit.startedAtMs = 0
  activeQuestionId.value = ''
  Object.keys(answerTimeline).forEach((key) => {
    delete answerTimeline[key]
  })
  markedQuestions.value = []
  interactionCount.value = 0
}

function getCurrentScrollY() {
  if (typeof window === 'undefined') return 0
  const numeric = Number(window.scrollY)
  return Number.isFinite(numeric) && numeric > 0 ? Math.round(numeric) : 0
}

async function loadReadingCoachPreference() {
  readingCoachSettingError.value = ''
  try {
    readingCoachEnabled.value = await readingCoachSettingsApi.getEnabled()
  } catch (error) {
    readingCoachEnabled.value = true
    console.warn('加载阅读 AI 教练设置失败，继续使用默认开启:', error)
  }
  if (!readingCoachEnabled.value) {
    resetReadingCoachState()
  }
}

function syncReadingCoachStateWithSetting(enabled) {
  if (!enabled) {
    resetReadingCoachState()
    return
  }
  if (submission.value) {
    hydrateReadingCoachFromSubmission(submission.value, {
      pendingIfMissing: true,
      successMessage: 'AI 复盘已载入',
      pendingMessage: 'AI 复盘待补全'
    })
  }
}

async function updateReadingCoachEnabled(enabled) {
  const normalized = Boolean(enabled)
  if (readingCoachSettingSaving.value || readingCoachEnabled.value === normalized) {
    return
  }
  const previous = readingCoachEnabled.value
  readingCoachEnabled.value = normalized
  readingCoachSettingError.value = ''
  syncReadingCoachStateWithSetting(normalized)
  readingCoachSettingSaving.value = true
  try {
    await readingCoachSettingsApi.updateEnabled(normalized)
  } catch (error) {
    readingCoachEnabled.value = previous
    syncReadingCoachStateWithSetting(previous)
    readingCoachSettingError.value = 'AI 教练设置保存失败，请稍后重试。'
    console.error('保存阅读 AI 教练设置失败:', error)
  } finally {
    readingCoachSettingSaving.value = false
  }
}

function restoreSubmittedMetadata(loadedSubmission) {
  markedQuestions.value = Array.isArray(loadedSubmission?.markedQuestions)
    ? loadedSubmission.markedQuestions.map(normalizeQuestionId).filter(Boolean)
    : []
  Object.keys(answerTimeline).forEach((key) => {
    delete answerTimeline[key]
  })
  const timeline = Array.isArray(loadedSubmission?.questionTimelineLite) ? loadedSubmission.questionTimelineLite : []
  timeline.forEach((entry) => {
    const questionId = normalizeQuestionId(entry?.questionId)
    if (!questionId) return
    const elapsedMs = Math.max(0, Number(entry.elapsedMs ?? entry.durationMs) || 0)
    answerTimeline[questionId] = {
      firstAnsweredAt: entry.firstAnsweredAt || null,
      lastAnsweredAt: entry.lastAnsweredAt || null,
      changeCount: Math.max(0, Number(entry.changeCount) || 0),
      visitCount: Math.max(0, Number(entry.visitCount) || 0),
      elapsedMs,
      lastFingerprint: getAnswerFingerprint(loadedSubmission.answers?.[questionId])
    }
  })
}

function resetAnswers() {
  if (readOnlyMode.value) {
    return
  }
  initializeReadingAnswers(payload.value, { prefillAnswerKey: isMemorizeMode.value })
  resetAttemptMetadata()
  if (activeSuiteSessionId.value && suiteTimerState.value) {
    applySuiteTimerState()
  } else {
    resetPracticeTimerClock()
  }
  startPracticeTimer()
  syncDomAnswers()
  showSnapshotMessage('已清空本页作答。')
}

function isMarkedQuestion(questionId) {
  const normalized = normalizeQuestionId(questionId)
  return Boolean(normalized && markedQuestions.value.includes(normalized))
}

function toggleMarkedQuestion(questionId) {
  if (readOnlyMode.value) return
  const normalized = normalizeQuestionId(questionId)
  if (!normalized) return
  markedQuestions.value = isMarkedQuestion(normalized)
    ? markedQuestions.value.filter((entry) => entry !== normalized)
    : [...markedQuestions.value, normalized]
  scheduleDraftAutosave()
}

async function recycleSubmittedAttempt() {
  if (!canRecycleSubmittedAttempt.value) {
    return
  }
  tauriAttemptId = ''
  submission.value = null
  clearSubmissionSnapshot()
  resetReadingCoachState()
  submitError.value = ''
  clearSnapshotMessage()
  resetAttemptMetadata()
  initializeReadingAnswers(payload.value, { prefillAnswerKey: isMemorizeMode.value })
  resetHighlightUiStateFromComposable()
  await nextTick()
  restoreHighlightsFromRecords([])
  syncDomAnswers()
  setReadOnlyDomControls(false)
  resetPracticeTimerClock()
  startPracticeTimer()
  showSnapshotMessage('已重置本篇练习，可重新作答。')
}

async function snapshotAnswers() {
  if (!canSnapshot.value) return false
  // Durable truth is Tauri SQLite draft; no Web Storage dual-write.
  try {
    await persistTauriDraft()
    showSnapshotMessage('作答快照已保存。')
    return true
  } catch (_) {
    showSnapshotMessage('作答快照保存失败，请重试。')
    return false
  }
}

function handleQuestionInput(event) {
  if (readOnlyMode.value) {
    return
  }
  const target = event.target
  if (!target || !target.name) {
    return
  }

  if (target.type === 'checkbox') {
    collectCheckboxGroup(target.name)
    scheduleDraftAutosave()
    return
  }

  const questionId = normalizeQuestionId(target.name)
  if (!questionId) return
  recordQuestionVisit(questionId)

  if (target.type === 'radio') {
    if (target.checked) {
      setAnswer(questionId, target.value)
      scheduleDraftAutosave()
    }
    return
  }

  setAnswer(questionId, target.value)
  scheduleDraftAutosave()
}

function collectCheckboxGroup(name) {
  const questionIds = expandQuestionSequence(name)
  if (!questionIds.length) {
    return
  }
  questionIds.forEach((questionId) => recordQuestionVisit(questionId))
  const checkedValues = Array.from(document.querySelectorAll(`input[type="checkbox"][name="${escapeCss(name)}"]`))
    .filter((input) => input.checked)
    .map((input) => String(input.value || '').trim())
    .filter(Boolean)
    .sort((left, right) => left.localeCompare(right, 'en'))

  if (questionIds.length === 1 && isMultiValueCheckbox(questionIds[0])) {
    setAnswer(questionIds[0], checkedValues)
    return
  }

  questionIds.forEach((questionId, index) => {
    setAnswer(questionId, checkedValues[index] || '')
  })
}

function syncDomAnswers() {
  getAnswerEntries().forEach(([questionId, value]) => {
    syncNativeControl(questionId, value)
  })
}

function syncNativeControl(questionId, explicitValue = getRawAnswer(questionId)) {
  if (typeof document === 'undefined') {
    return
  }
  const interaction = getInteraction(questionId)
  if (interaction?.control === 'dragdrop') {
    syncDropzoneControl(questionId, explicitValue)
    return
  }
  const names = new Set(resolveAnswerAliases(questionId))
  if (interaction?.name) {
    names.add(interaction.name)
  }

  if (interaction?.control === 'checkbox' && interaction.name) {
    const groupIds = expandQuestionSequence(interaction.name)
    const selectedValues = new Set()
    groupIds.forEach((id) => {
      const value = getRawAnswer(id)
      if (Array.isArray(value)) {
        value.map((entry) => String(entry || '').trim()).filter(Boolean).forEach((entry) => selectedValues.add(entry))
        return
      }
      const normalized = String(value || '').trim()
      if (normalized) {
        selectedValues.add(normalized)
      }
    })
    document.querySelectorAll(`input[type="checkbox"][name="${escapeCss(interaction.name)}"]`).forEach((input) => {
      input.checked = selectedValues.has(String(input.value || '').trim())
      syncNativeReadOnly(input)
    })
    return
  }

  names.forEach((name) => {
    const escaped = escapeCss(name)
    document.querySelectorAll(`input[type="radio"][name="${escaped}"]`).forEach((input) => {
      input.checked = String(input.value || '').trim() === String(explicitValue || '').trim()
      syncNativeReadOnly(input)
    })
    document.querySelectorAll(`input[type="text"][name="${escaped}"], textarea[name="${escaped}"]`).forEach((input) => {
      input.value = String(explicitValue || '')
      syncNativeReadOnly(input)
    })
  })
}

function syncNativeReadOnly(control) {
  const readOnly = readOnlyMode.value
  control.disabled = readOnly
  control.tabIndex = readOnly ? -1 : 0
  control.setAttribute('aria-disabled', readOnly ? 'true' : 'false')
}

function scheduleDraftAutosave() {
  if (!isTauriRuntime() || readOnlyMode.value) {
    return
  }
  if (draftAutosaveTimer) clearTimeout(draftAutosaveTimer)
  draftAutosaveTimer = setTimeout(() => {
    draftAutosaveTimer = null
    void persistTauriDraft().catch((error) => {
      console.warn('阅读草稿自动保存失败', error)
    })
  }, 800)
}

async function persistTauriDraft() {
  if (!isTauriRuntime() || !asset.value?.id || readOnlyMode.value) return
  if (activeSuiteSessionId.value) {
    const { result } = await saveSuitePassageDraft({
      suiteId: activeSuiteSessionId.value,
      assetId: asset.value.id,
      assetRevision: asset.value.schemaVersion ?? null,
      assetFingerprint: asset.value.fingerprint || null,
      answers: snapshotAnswerMap(),
      markedQuestions: markedQuestions.value.slice(),
      questionTimeline: buildPersistedQuestionTimeline(),
      titleSnapshot: asset.value.title || asset.value.name || null,
      timerSnapshot: getPracticeTimerSnapshot()
    })
    suiteSession.value = result?.suiteSession || suiteSession.value
    tauriAttemptId = String(result?.attempt?.id || tauriAttemptId || '')
    return
  }
  if (isEndlessMode.value && activeEndlessSessionId.value) {
    const { result } = await saveEndlessPassageDraft({
      sessionId: activeEndlessSessionId.value,
      assetId: asset.value.id,
      assetRevision: asset.value.schemaVersion ?? null,
      assetFingerprint: asset.value.fingerprint || null,
      answers: snapshotAnswerMap(),
      markedQuestions: markedQuestions.value.slice(),
      questionTimeline: buildPersistedQuestionTimeline(),
      titleSnapshot: asset.value.title || asset.value.name || null,
      timerSnapshot: getPracticeTimerSnapshot()
    })
    tauriAttemptId = String(result?.attempt?.id || tauriAttemptId || '')
    return
  }
  if (!tauriAttemptId) tauriAttemptId = readingAttempt.newAttemptId()
  await readingAttempt.persistDraft({
    attemptId: tauriAttemptId,
    assetId: asset.value.id,
    assetRevision: asset.value.schemaVersion ?? null,
    assetFingerprint: asset.value.fingerprint || null,
    answers: snapshotAnswerMap(),
    markedQuestions: markedQuestions.value.slice(),
    questionTimeline: buildPersistedQuestionTimeline(),
    titleSnapshot: asset.value.title || asset.value.name || null,
    timerSnapshot: getPracticeTimerSnapshot()
  })
}

function toggleTimer() {
  togglePracticeTimer()
  if (isTauriRuntime() && !readOnlyMode.value) {
    void persistTauriDraft().catch((error) => {
      console.warn('切换阅读计时时保存草稿失败', error)
    })
  }
}

const modeFlow = useReadingModeFlow({
  router,
  routeQuerySource: () => route.query,
  assetSource: () => asset.value,
  submission,
  suiteSession,
  submitting,
  leaving,
  submitError,
  snapshotMessage,
  endlessCountdown,
  endlessNextAssetId,
  activeMemorizeAttemptId,
  activeSuiteSessionId,
  activeEndlessSessionId,
  isEndlessMode,
  isMemorizeMode,
  reviewMode,
  canSubmit,
  canRecycleSubmittedAttempt,
  readingCoachEnabled,
  suiteAutoAdvance,
  suiteSequence,
  currentSuitePassageIndex,
  returnRoute,
  getAttemptId: () => tauriAttemptId,
  setAttemptId: (id) => { tauriAttemptId = id },
  newAttemptId: () => readingAttempt.newAttemptId(),
  submitSingleAttempt: (input) => readingAttempt.submit(input),
  snapshotAnswerMap,
  markedQuestions,
  interactionCount,
  snapshotHighlights,
  highlightSnapshot,
  buildQuestionTimelineLite,
  buildPersistedQuestionTimeline,
  getPracticeTimerSnapshot,
  resolvePracticeTiming,
  getCurrentScrollY,
  flushActiveQuestionVisit,
  stopPracticeTimer,
  startPracticeTimer,
  setPracticeTimerElapsedSeconds,
  assignAnswer,
  syncDomAnswers,
  setReadOnlyDomControls,
  restoreHighlightsFromRecords,
  setReadingCoachOpen,
  runAutomaticReviewCoach,
  resetAnswers,
  recycleSubmittedAttempt
})

function submitAnswers() { return modeFlow.submitAnswers() }
function snapshotSubmission() { return modeFlow.snapshotSubmission() }
function clearSubmissionSnapshot() { return modeFlow.clearSubmissionSnapshot() }
function clearEndlessTimer() { return modeFlow.clearEndlessTimer() }
function findSuiteReviewNavigationTarget(direction) { return modeFlow.findSuiteReviewNavigationTarget(direction) }
function navigateSuiteReview(direction) { return modeFlow.navigateSuiteReview(direction) }
function goToNextEndlessAsset() { return modeFlow.goToNextEndlessAsset() }
function stopEndlessMode() { return modeFlow.stopEndlessMode() }
function handleLeave() { return modeFlow.handleLeave() }
function reconcileEndlessRoute() { return modeFlow.reconcileEndlessRoute() }
function handleResetButton() { return modeFlow.handleResetButton() }
function handlePrimaryButton() { return modeFlow.handlePrimaryButton() }
function ensureMemorizeSession() { return modeFlow.ensureMemorizeSession() }
function finishActiveMemorizeSession() { return modeFlow.finishActiveMemorizeSession() }

function startDividerDrag(event) {
  if (!event || (Number.isFinite(Number(event.button)) && event.button > 0)) {
    return
  }
  const shell = document.querySelector('.reading-workspace.shell')
  if (!shell) return
  event.preventDefault()
  dividerDragging.value = true
  dividerPointerId = event.pointerId
  event.currentTarget?.setPointerCapture?.(event.pointerId)
  document.addEventListener('pointermove', handleDividerDrag)
  document.addEventListener('pointerup', stopDividerDrag)
  document.addEventListener('pointercancel', stopDividerDrag)
  handleDividerDrag(event)
}

function handleDividerDrag(event) {
  if (!dividerDragging.value || !event) {
    return
  }
  const shell = document.querySelector('.reading-workspace.shell')
  if (!shell) return
  const rect = shell.getBoundingClientRect()
  if (!rect.width) return
  const percent = ((event.clientX - rect.left) / rect.width) * 100
  leftPanePercent.value = Math.max(34, Math.min(66, percent))
}

function stopDividerDrag(event) {
  if (!dividerDragging.value) {
    return
  }
  dividerDragging.value = false
  const divider = document.getElementById('reading-divider')
  try {
    if (divider && dividerPointerId != null) {
      divider.releasePointerCapture?.(dividerPointerId)
    } else if (divider && event?.pointerId != null) {
      divider.releasePointerCapture?.(event.pointerId)
    }
  } catch (_) {}
  dividerPointerId = null
  removeDividerDragListeners()
}

function removeDividerDragListeners() {
  document.removeEventListener('pointermove', handleDividerDrag)
  document.removeEventListener('pointerup', stopDividerDrag)
  document.removeEventListener('pointercancel', stopDividerDrag)
}

function handleDividerKeydown(event) {
  const step = event.shiftKey ? 8 : 3
  if (event.key === 'ArrowLeft') {
    event.preventDefault()
    leftPanePercent.value = Math.max(34, leftPanePercent.value - step)
  } else if (event.key === 'ArrowRight') {
    event.preventDefault()
    leftPanePercent.value = Math.min(66, leftPanePercent.value + step)
  } else if (event.key === 'Home') {
    event.preventDefault()
    leftPanePercent.value = 40
  } else if (event.key === 'End') {
    event.preventDefault()
    leftPanePercent.value = 60
  }
}

function applyMemorizeStudyLayer() {
  clearMemorizeLocatorHighlights()
  if (!isMemorizeMode.value || !payload.value?.answerKey) {
    return
  }
  Object.entries(payload.value.answerKey).forEach(([questionId, answer]) => {
    const text = formatReviewAnswer(answer)
    if (text) {
      applyMemorizeLocatorHighlights(questionId, text)
    }
  })
  setReadOnlyDomControls(true)
}

function applyMemorizeLocatorHighlights(questionId, answerText) {
  const root = document.getElementById('left')
  if (!root) return
  const tokens = String(answerText || '')
    .split(/[;,/|]/)
    .map((entry) => entry.replace(/^\s*[A-Z]\.\s*/, '').trim())
    .filter((entry) => entry.length >= 3)
    .slice(0, 3)
  tokens.forEach((token) => {
    wrapTextMatches(root, token, {
      className: 'memorize-locator-highlight',
      attrs: {
        'data-memorize-question-id': questionId
      },
      limit: 1,
      skipSelector: '.hl, .memorize-locator-highlight'
    })
  })
}

function clearMemorizeLocatorHighlights() {
  if (typeof document === 'undefined') return
  document.querySelectorAll('.memorize-locator-highlight').forEach((node) => {
    const parent = node.parentNode
    if (!parent) return
    while (node.firstChild) {
      parent.insertBefore(node.firstChild, node)
    }
    parent.removeChild(node)
    parent.normalize()
  })
}

function wrapTextMatches(root, needle, options = {}) {
  const text = normalizeComparableText(needle)
  if (!root || text.length < 3) {
    return []
  }
  const matches = []
  const className = options.className || 'hl'
  const attrs = options.attrs || {}
  const limit = Math.max(1, Number(options.limit) || 20)
  const skipSelector = options.skipSelector || '.hl'
  const nodes = getTextNodes(root)
  for (let index = 0; index < nodes.length && matches.length < limit; index += 1) {
    let current = nodes[index]
    if (current.parentElement?.closest?.(skipSelector)) {
      continue
    }
    while (current && matches.length < limit) {
      const source = current.nodeValue || ''
      const hit = source.toLowerCase().indexOf(text.toLowerCase())
      if (hit < 0) break
      const matchedNode = current.splitText(hit)
      const remainder = matchedNode.splitText(text.length)
      const span = document.createElement('span')
      span.className = className
      Object.entries(attrs).forEach(([key, value]) => {
        if (value != null) {
          span.setAttribute(key, String(value))
        }
      })
      matchedNode.parentNode.insertBefore(span, matchedNode)
      span.appendChild(matchedNode)
      matches.push(span)
      current = remainder
    }
  }
  return matches
}

function readSelectedContext() {
  if (typeof window === 'undefined' || typeof window.getSelection !== 'function') {
    return null
  }
  const selection = window.getSelection()
  const text = selection && typeof selection.toString === 'function'
    ? String(selection.toString() || '').trim()
    : ''
  if (!text) {
    return null
  }
  const element = resolveSelectionElement(selection)
  const questionNumbers = collectSelectedQuestionNumbers(element)
  const paragraphLabels = collectSelectedParagraphLabels(element)
  const scope = element?.closest?.('.question-panel, .question-group, [data-answer-question-id], [data-review-question-id]')
    ? 'question'
    : 'passage'
  return {
    text: text.slice(0, 500),
    scope,
    questionNumbers,
    paragraphLabels
  }
}

function resolveSelectionElement(selection) {
  const nodes = [selection?.anchorNode, selection?.focusNode].filter(Boolean)
  for (const node of nodes) {
    const element = node.nodeType === 3 ? node.parentElement : node
    if (element && typeof element.closest === 'function') {
      return element
    }
  }
  return null
}

function normalizeSelectedQuestionNumber(value) {
  const raw = String(value || '').trim()
  if (!raw) return ''
  const exactNumber = raw.match(/^\d+$/)
  const qNumber = raw.match(/\bq(\d+)\b/i) || raw.match(/^q(\d+)/i)
  const numeric = exactNumber?.[0] || qNumber?.[1] || ''
  if (!numeric) return ''
  const questionId = normalizeQuestionId(`q${Number(numeric)}`)
  return String(getDisplayLabel(questionId) || numeric).replace(/^q/i, '').trim()
}

function collectSelectedQuestionNumbers(element) {
  if (!element || typeof element.closest !== 'function') {
    return []
  }
  const candidates = []
  const direct = element.closest('[data-answer-question-id], [data-review-question-id], [data-question-ids], [data-question], [data-question-id], [name], [id]')
  if (direct?.dataset?.questionIds) {
    candidates.push(...String(direct.dataset.questionIds).split(','))
  }
  if (direct?.dataset?.answerQuestionId) {
    candidates.push(direct.dataset.answerQuestionId)
  }
  if (direct?.dataset?.reviewQuestionId) {
    candidates.push(direct.dataset.reviewQuestionId)
  }
  if (direct?.dataset?.question) {
    candidates.push(direct.dataset.question)
  }
  if (direct?.dataset?.questionId) {
    candidates.push(direct.dataset.questionId)
  }
  const name = direct?.getAttribute?.('name')
  if (name) candidates.push(name)
  if (direct?.id) candidates.push(direct.id)

  const group = element.closest('[data-question-ids]')
  if (group?.dataset?.questionIds) {
    candidates.push(...String(group.dataset.questionIds).split(','))
  }
  return Array.from(new Set(candidates.map(normalizeSelectedQuestionNumber).filter(Boolean)))
}

function collectSelectedParagraphLabels(element) {
  if (!element || typeof element.closest !== 'function') {
    return []
  }
  const labels = []
  const paragraph = element.closest('[data-paragraph], [data-paragraph-label], .passage-block, .paragraph-wrapper')
  if (paragraph?.dataset?.paragraph) {
    labels.push(paragraph.dataset.paragraph)
  }
  if (paragraph?.dataset?.paragraphLabel) {
    labels.push(paragraph.dataset.paragraphLabel)
  }
  const text = String(paragraph?.textContent || element.textContent || '').trim()
  const match = text.match(/^(?:paragraph\s*)?([A-H])\b/i)
  if (match?.[1]) {
    labels.push(match[1])
  }
  return Array.from(new Set(labels.map((item) => String(item || '').replace(/^paragraph\s*/i, '').trim().toUpperCase()).filter(Boolean)))
}

function getReviewEntry(questionId) {
  return submission.value?.answerComparison?.[questionId] || null
}

function getReviewClass(questionId) {
  const entry = getReviewEntry(questionId)
  if (!entry) return ''
  if (entry.isCorrect === true) return 'review-correct'
  if (entry.isCorrect === false) return 'review-incorrect'
  return 'review-neutral'
}

function getLegacyNavStatus(questionId) {
  const entry = getReviewEntry(questionId)
  if (entry?.isCorrect === true) return 'correct'
  if (entry?.isCorrect === false) return 'incorrect'
  return hasAnswer(questionId) ? 'answered' : ''
}

function isActiveQuestion(questionId) {
  const normalized = normalizeQuestionId(questionId)
  return Boolean(normalized && activeQuestionId.value === normalized)
}

function getLegacyResultClass(questionId) {
  const entry = getReviewEntry(questionId)
  if (entry?.isCorrect === true) return 'result-correct'
  if (entry?.isCorrect === false) return 'result-incorrect'
  return ''
}

function scrollToQuestion(questionId) {
  const normalized = normalizeQuestionId(questionId)
  if (!normalized) return
  recordQuestionVisit(normalized)
  const aliases = resolveAnswerAliases(normalized).map((entry) => escapeCss(entry))
  const directSelectors = aliases.flatMap((alias) => [
    `#${alias}-anchor`,
    `.question-panel [data-question="${alias}"]`,
    `.question-panel [data-question-id="${alias}"]`,
    `.question-panel [name="${alias}"]`
  ])
  const directTarget = directSelectors
    .map((selector) => document.querySelector(selector))
    .find(Boolean)
  if (directTarget) {
    directTarget.scrollIntoView({ behavior: 'smooth', block: 'center' })
    return
  }
  const groupTarget = Array.from(document.querySelectorAll('.question-panel [data-question-ids]')).find((group) => {
    const ids = String(group.dataset.questionIds || '')
      .split(',')
      .map((entry) => normalizeQuestionId(entry))
      .filter(Boolean)
    return ids.includes(normalized)
  })
  groupTarget?.scrollIntoView?.({ behavior: 'smooth', block: 'start' })
}

function getReviewLabel(questionId) {
  const entry = getReviewEntry(questionId)
  if (!entry) return ''
  if (entry.isCorrect === true) return '正确'
  if (entry.isCorrect === false) return '错误'
  return '未判定'
}

function formatReviewAnswer(value) {
  if (Array.isArray(value)) {
    return value.map((entry) => String(entry || '').trim()).filter(Boolean).join(' / ')
  }
  return String(value == null ? '' : value).trim()
}

function formatDuration(seconds) {
  const totalSeconds = Math.max(0, Number(seconds) || 0)
  const minutes = Math.floor(totalSeconds / 60)
  const rest = totalSeconds % 60
  return minutes > 0 ? `${minutes}分${rest}秒` : `${rest}秒`
}

function formatDensity(value) {
  const numeric = Number(value)
  return Number.isFinite(numeric) ? numeric.toFixed(1) : '0.0'
}

function getSeverityLabel(severity) {
  const labels = {
    high: '高',
    medium: '中',
    low: '低'
  }
  return labels[severity] || '提示'
}

function getOrCreateQuestionTimelineEntry(questionId) {
  const normalized = normalizeQuestionId(questionId)
  if (!normalized) return null
  const current = answerTimeline[normalized] || {
    firstAnsweredAt: null,
    lastAnsweredAt: null,
    changeCount: 0,
    visitCount: 0,
    elapsedMs: 0,
    lastFingerprint: getAnswerFingerprint(getRawAnswer(normalized))
  }
  current.changeCount = Math.max(0, Number(current.changeCount) || 0)
  current.visitCount = Math.max(0, Number(current.visitCount) || 0)
  current.elapsedMs = Math.max(0, Number(current.elapsedMs) || 0)
  answerTimeline[normalized] = current
  return current
}

function flushActiveQuestionVisit(nowMs = Date.now()) {
  const questionId = activeQuestionVisit.questionId
  const startedAtMs = Number(activeQuestionVisit.startedAtMs) || 0
  if (!questionId || !startedAtMs || nowMs <= startedAtMs) {
    return
  }
  const entry = getOrCreateQuestionTimelineEntry(questionId)
  if (!entry) return
  entry.elapsedMs = Math.max(0, Number(entry.elapsedMs) || 0) + Math.max(0, nowMs - startedAtMs)
  activeQuestionVisit.startedAtMs = nowMs
}

function recordQuestionVisit(questionId) {
  if (readOnlyMode.value) {
    return
  }
  const normalized = normalizeQuestionId(questionId)
  if (!normalized) return
  activeQuestionId.value = normalized
  const nowMs = Date.now()
  if (activeQuestionVisit.questionId && activeQuestionVisit.questionId !== normalized) {
    flushActiveQuestionVisit(nowMs)
  }
  const entry = getOrCreateQuestionTimelineEntry(normalized)
  if (!entry) return
  if (activeQuestionVisit.questionId !== normalized) {
    entry.visitCount += 1
    activeQuestionVisit.questionId = normalized
  }
  activeQuestionVisit.startedAtMs = nowMs
}

function recordInteraction() {
  if (!readOnlyMode.value) {
    interactionCount.value += 1
  }
}

function recordAnswerTimeline(questionId, previousFingerprint, nextFingerprint) {
  const changed = previousFingerprint !== nextFingerprint
  if (!changed) {
    return
  }
  recordInteraction()
  const now = new Date().toISOString()
  recordQuestionVisit(questionId)
  const current = getOrCreateQuestionTimelineEntry(questionId)
  if (!current) return
  if (nextFingerprint && !current.firstAnsweredAt) {
    current.firstAnsweredAt = now
  }
  if (nextFingerprint) {
    current.lastAnsweredAt = now
  }
  if (current.lastFingerprint && current.lastFingerprint !== nextFingerprint) {
    current.changeCount += 1
  }
  current.lastFingerprint = nextFingerprint
  answerTimeline[questionId] = current
}

function buildQuestionTimelineLite() {
  flushActiveQuestionVisit()
  const order = Array.isArray(payload.value?.questionOrder) ? payload.value.questionOrder : []
  return order.map((questionId) => {
    const entry = answerTimeline[questionId] || {}
    const elapsedMs = Math.max(0, Math.round(Number(entry.elapsedMs) || 0))
    return {
      questionId,
      displayLabel: getDisplayLabel(questionId),
      firstAnsweredAt: entry.firstAnsweredAt || null,
      lastAnsweredAt: entry.lastAnsweredAt || null,
      changeCount: Math.max(0, Number(entry.changeCount) || 0),
      visitCount: Math.max(0, Number(entry.visitCount) || 0),
      elapsedMs,
      durationMs: elapsedMs
    }
  })
}

function buildPersistedQuestionTimeline(timeline = buildQuestionTimelineLite()) {
  return timeline.map((entry) => ({
    questionId: entry.questionId,
    changeCount: entry.changeCount,
    visitCount: entry.visitCount,
    elapsedMs: entry.elapsedMs,
    answeredAt: entry.lastAnsweredAt || entry.firstAnsweredAt || null
  }))
}

function normalizeOfficialQuestionExplanationSection(section, index) {
  const rawItems = Array.isArray(section?.items) ? section.items : []
  const items = rawItems
    .map((item) => {
      const questionNumber = Number(item?.questionNumber)
      const text = String(item?.text || '').trim()
      if (!Number.isFinite(questionNumber) || !text) return null
      return {
        questionNumber,
        questionId: normalizeQuestionId(item?.questionId || questionNumber),
        text
      }
    })
    .filter(Boolean)
  const text = String(section?.text || '').trim()
  if (!items.length && !text) return null
  return {
    sectionTitle: String(section?.sectionTitle || `题目讲解 ${index + 1}`).trim(),
    mode: String(section?.mode || '').trim(),
    questionRange: normalizeOfficialQuestionRange(section?.questionRange),
    text,
    items
  }
}

function normalizeOfficialQuestionRange(range) {
  const start = Number(range?.start)
  const end = Number(range?.end)
  return Number.isFinite(start) && Number.isFinite(end) ? { start, end } : null
}

function getQuestionOfficialNumber(questionId) {
  const normalized = normalizeQuestionId(questionId)
  const displayNumber = Number(String(payload.value?.questionDisplayMap?.[normalized] || '').match(/\d+/)?.[0])
  if (Number.isFinite(displayNumber)) return displayNumber
  const internalNumber = Number(String(normalized || '').replace(/^q/i, ''))
  return Number.isFinite(internalNumber) ? internalNumber : null
}

function getGroupOfficialQuestionNumbers(group) {
  return (Array.isArray(group?.questionIds) ? group.questionIds : [])
    .map((questionId) => getQuestionOfficialNumber(questionId))
    .filter((value) => Number.isFinite(value))
}

function sectionOverlapsOfficialNumbers(section, questionNumbers) {
  if (!questionNumbers.length) return false
  const itemNumbers = new Set((section.items || []).map((item) => Number(item.questionNumber)).filter((value) => Number.isFinite(value)))
  if (questionNumbers.some((value) => itemNumbers.has(value))) return true
  const range = section.questionRange
  if (!range) return false
  return questionNumbers.some((value) => value >= range.start && value <= range.end)
}

function getGroupOfficialExplanations(group) {
  if (!reviewMode.value) return []
  const questionNumbers = getGroupOfficialQuestionNumbers(group)
  if (!questionNumbers.length) return []
  const splitMode = EXPLANATION_SPLIT_KINDS.has(String(group?.kind || ''))
  return officialQuestionExplanationSections.value
    .filter((section) => sectionOverlapsOfficialNumbers(section, questionNumbers))
    .map((section) => {
      const matchedItems = (section.items || []).filter((item) => questionNumbers.includes(Number(item.questionNumber)))
      const groupMode = section.mode === 'group' || (!splitMode && section.text)
      return {
        ...section,
        text: groupMode || (!matchedItems.length && section.text) ? section.text : '',
        items: groupMode ? [] : matchedItems
      }
    })
    .filter((section) => section.text || section.items.length)
}


function resolveAnswerAliases(questionId) {
  return resolveAnswerAliasesFromIds(questionId, payload.value?.questionDisplayMap || null)
}

function getGroupRange(group) {
  const ids = Array.isArray(group.questionIds) ? group.questionIds : []
  if (!ids.length) return '题目'
  const labels = ids.map((questionId) => getDisplayLabel(questionId)).filter(Boolean)
  if (labels.length <= 1) return `第 ${labels[0] || ''} 题`
  return `第 ${labels[0]}-${labels[labels.length - 1]} 题`
}

function getQuestionKindLabel(kind) {
  const labels = {
    matching: '匹配题',
    table_completion: '表格题',
    summary_completion: '摘要填空',
    multi_choice: '多选题',
    true_false_not_given: '判断题'
  }
  return labels[kind] || kind || '题组'
}

</script>

<style>
.reading-page {
  display: grid;
  gap: 20px;
  animation: rise-in 0.45s var(--ease-smooth);
}

.reading-header {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: 24px;
  padding: 10px 2px 4px;
}

.reading-header h1 {
  margin-top: 6px;
  font-size: clamp(1.85rem, 2.6vw, 3rem);
}

.reading-header p {
  max-width: 760px;
  margin-top: 8px;
  color: var(--text-secondary);
}

.eyebrow,
.panel-kicker {
  color: var(--primary-color);
  font-size: 0.76rem;
  font-weight: 700;
  letter-spacing: 0.14em;
  text-transform: uppercase;
}

.reading-header-actions,
.answer-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}

.reading-workspace {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(300px, 0.28fr);
  gap: 20px;
  align-items: start;
}

.reading-main {
  display: grid;
  gap: 20px;
  min-width: 0;
}

.reading-panel,
.answer-panel {
  padding: 22px;
}

.panel-heading,
.answer-panel-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 18px;
}

.panel-heading strong,
.answer-count {
  color: var(--text-secondary);
  font-size: 0.9rem;
}

.reading-html {
  color: var(--text-primary);
}

.reading-html h2,
.reading-html h3,
.reading-html h4,
.reading-html h5 {
  margin: 18px 0 10px;
  font-family: var(--font-family-display);
}

.reading-html p {
  margin: 10px 0;
}

.reading-html table {
  width: 100%;
  border-collapse: collapse;
  margin: 14px 0;
  background: var(--atlas-glass);
}

.reading-html th,
.reading-html td {
  padding: 9px 10px;
  border: 1px solid rgba(41, 39, 56, 0.12);
  vertical-align: top;
}

.reading-html input[type="radio"],
.reading-html input[type="checkbox"] {
  width: 18px;
  height: 18px;
  accent-color: var(--primary-color);
}

.reading-html .paragraph-dropzone,
.reading-html .match-dropzone,
.reading-html .drop-target-summary {
  min-height: 36px;
  margin: 8px 0;
  padding: 8px 10px;
  border: 1px dashed var(--atlas-accent-ring);
  border-radius: var(--radius-md);
  background: var(--atlas-glass);
  transition: border-color 0.16s ease, background 0.16s ease, box-shadow 0.16s ease;
}

.reading-html .dropzone-filled {
  border-style: solid;
  border-color: var(--atlas-success);
  background: var(--atlas-accent-soft);
}

.reading-html .drag-over,
.dragdrop-slot.drag-over {
  border-color: var(--primary-color);
  box-shadow: 0 0 0 3px var(--atlas-accent-soft);
}

.reading-html .pool-items,
.reading-html .options-pool,
.reading-html .headings-pool {
  transition: background 0.16s ease, box-shadow 0.16s ease;
}

.reading-html .pool-items.drag-over,
.reading-html .options-pool.drag-over,
.reading-html .headings-pool.drag-over {
  background: var(--atlas-accent-soft);
  box-shadow: inset 0 0 0 1px var(--atlas-accent-ring);
}

.question-group {
  padding: 18px 0;
  border-top: 1px solid var(--lg-border-subtle);
}

.question-group:first-of-type {
  padding-top: 0;
  border-top: 0;
}

.group-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-bottom: 10px;
  color: var(--text-muted);
  font-size: 0.8rem;
  font-weight: 700;
}

.snapshot-message {
  margin-top: 12px;
  color: var(--warning-color);
  font-size: 0.86rem;
}

.answer-panel {
  position: sticky;
  top: 92px;
  display: grid;
  gap: 16px;
}

.answer-panel h2 {
  font-size: 1.25rem;
}

.answer-status {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 10px;
}

.answer-status > div {
  padding: 10px;
  border: 1px solid var(--lg-border-subtle);
  border-radius: var(--radius-md);
  background: var(--atlas-glass);
}

.answer-status span {
  display: block;
  color: var(--text-muted);
  font-size: 0.74rem;
}

.answer-status strong {
  display: block;
  margin-top: 2px;
  overflow-wrap: anywhere;
  font-size: 0.92rem;
}

.suite-progress-mini {
  display: grid;
  gap: 3px;
  padding: 10px;
  border: 1px solid var(--atlas-accent-ring);
  border-radius: var(--radius-md);
  background: var(--atlas-accent-soft);
}

.suite-progress-mini span {
  color: var(--text-muted);
  font-size: 0.74rem;
}

.suite-progress-mini strong {
  color: var(--text-primary);
  font-size: 0.92rem;
}

.answer-list {
  display: grid;
  gap: 8px;
  max-height: min(62vh, 720px);
  overflow: auto;
  padding-right: 4px;
}

.answer-item {
  display: grid;
  grid-template-columns: 42px minmax(0, 1fr);
  gap: 8px;
  align-items: start;
}

.mark-question-button {
  min-height: 22px;
  border: 1px solid rgba(41, 39, 56, 0.14);
  border-radius: var(--radius-sm);
  background: var(--atlas-glass);
  color: var(--text-muted);
  cursor: pointer;
  font: inherit;
  font-size: 0.74rem;
  font-weight: 800;
}

.mark-question-button.active {
  border-color: var(--atlas-accent-ring);
  background: var(--atlas-accent-soft);
  color: var(--primary-color);
}

.mark-question-button:disabled {
  cursor: default;
  opacity: 0.7;
}

.answer-checkbox-list {
  display: grid;
  gap: 6px;
}

.answer-checkbox-option {
  display: flex;
  align-items: center;
  gap: 8px;
  min-height: 30px;
  color: var(--text-secondary);
  font-size: 0.88rem;
}

.answer-checkbox-option input {
  width: 16px;
  height: 16px;
  accent-color: var(--primary-color);
}

.answer-dragdrop {
  display: grid;
  gap: 8px;
  min-width: 0;
}

.dragdrop-slot {
  display: flex;
  align-items: center;
  min-height: 36px;
  padding: 5px 7px;
  border: 1px dashed rgba(41, 39, 56, 0.2);
  border-radius: var(--radius-md);
  color: var(--text-muted);
  background: var(--atlas-glass);
  font-size: 0.84rem;
}

.dragdrop-slot.filled {
  border-style: solid;
  border-color: var(--atlas-success);
  background: var(--atlas-accent-soft);
}

.dragdrop-options {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.dragdrop-chip,
.reading-html .dragdrop-chip {
  min-height: 30px;
  max-width: 100%;
  padding: 5px 9px;
  border: 1px solid rgba(41, 39, 56, 0.14);
  border-radius: var(--radius-md);
  background: var(--atlas-glass-elevated);
  color: var(--text-secondary);
  cursor: grab;
  font: inherit;
  font-size: 0.82rem;
  line-height: 1.35;
  overflow-wrap: anywhere;
  text-align: left;
}

.dragdrop-chip:disabled,
.reading-html .dragdrop-chip:disabled {
  cursor: default;
  opacity: 0.76;
}

.dragdrop-chip-assigned,
.reading-html .dragdrop-chip-assigned {
  border-color: var(--atlas-success);
  background: rgba(35, 143, 91, 0.14);
  color: var(--text-primary);
}

.dragdrop-option.selected {
  border-color: var(--atlas-success);
  background: rgba(35, 143, 91, 0.14);
}

.dragdrop-option:disabled:not(.selected) {
  cursor: not-allowed;
  opacity: 0.38;
}

.dragdrop-chip.dragging,
.reading-html .dragging {
  opacity: 0.58;
}

.dragdrop-select {
  width: 100%;
}

.score-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 10px;
  margin-bottom: 18px;
}

.score-grid > div {
  padding: 12px;
  border: 1px solid var(--lg-border-subtle);
  border-radius: var(--radius-md);
  background: var(--atlas-glass);
}

.score-grid span {
  display: block;
  color: var(--text-muted);
  font-size: 0.75rem;
}

.score-grid strong {
  display: block;
  margin-top: 4px;
  font-size: 1.1rem;
}

.review-analysis {
  display: grid;
  gap: 16px;
  margin: 4px 0 20px;
  padding: 16px 0 2px;
  border-top: 1px solid var(--lg-border-subtle);
  border-bottom: 1px solid var(--lg-border-subtle);
}

.llm-review-status {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
  flex-wrap: wrap;
}

.llm-review-status > strong {
  min-width: 0;
  flex: 1;
}

.llm-review-retry {
  flex: 0 0 auto;
}

.analysis-strip {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 10px;
}

.analysis-strip > div {
  min-width: 0;
  padding: 10px 0;
}

.analysis-strip span {
  display: block;
  color: var(--text-muted);
  font-size: 0.75rem;
}

.analysis-strip strong {
  display: block;
  margin-top: 3px;
  color: var(--text-primary);
  font-size: 1.05rem;
}

.analysis-body {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 18px;
}

.llm-analysis-body {
  align-items: start;
}

.llm-question-analysis-list {
  grid-column: 1 / -1;
  display: grid;
  gap: 10px;
  min-width: 0;
}

.llm-question-analysis {
  display: grid;
  grid-template-columns: 52px minmax(0, 1fr);
  gap: 10px;
  padding: 10px 0;
  border-top: 1px solid var(--lg-border-subtle);
  color: var(--text-secondary);
  font-size: 0.86rem;
}

.llm-question-analysis > strong {
  color: var(--text-primary);
}

.llm-question-analysis dl {
  display: grid;
  grid-template-columns: 72px minmax(0, 1fr);
  gap: 6px 10px;
  margin: 0;
}

.llm-question-analysis dt {
  color: var(--text-muted);
}

.llm-question-analysis dd {
  min-width: 0;
  margin: 0;
  overflow-wrap: anywhere;
}

.analysis-body h3 {
  margin-bottom: 8px;
  font-size: 0.94rem;
}

.analysis-list {
  display: grid;
  gap: 8px;
  margin: 0;
  padding: 0;
  list-style: none;
}

.analysis-list li {
  display: grid;
  grid-template-columns: 42px minmax(0, 1fr);
  gap: 8px;
  align-items: start;
  color: var(--text-secondary);
  font-size: 0.88rem;
}

.analysis-list strong {
  color: var(--text-primary);
}

.analysis-kind-bars {
  display: grid;
  gap: 8px;
  padding-bottom: 14px;
}

.kind-bar-row {
  display: grid;
  grid-template-columns: minmax(92px, 0.24fr) minmax(0, 1fr) 54px;
  gap: 10px;
  align-items: center;
  color: var(--text-secondary);
  font-size: 0.86rem;
}

.kind-bar-track {
  height: 8px;
  overflow: hidden;
  border-radius: 999px;
  background: rgba(41, 39, 56, 0.1);
}

.kind-bar-track i {
  display: block;
  height: 100%;
  border-radius: inherit;
  background: var(--success-color);
}

.review-table,
.results-table {
  width: 100%;
  border-collapse: collapse;
  overflow-wrap: anywhere;
}

.review-table th,
.review-table td,
.results-table th,
.results-table td {
  padding: 10px;
  border-bottom: 1px solid var(--lg-border-subtle);
  text-align: left;
  vertical-align: top;
}

.review-table th,
.results-table th {
  color: var(--text-muted);
  font-size: 0.78rem;
  font-weight: 700;
}

.review-correct td:last-child,
.result-correct {
  color: var(--success-color);
  font-weight: 700;
}

.review-incorrect td:last-child,
.result-incorrect {
  color: var(--danger-color);
  font-weight: 700;
}

@media (max-width: 1100px) {
  .reading-header,
  .reading-workspace {
    grid-template-columns: 1fr;
  }

  .reading-header {
    align-items: stretch;
    flex-direction: column;
  }

  .answer-panel {
    position: static;
  }

  .score-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .analysis-strip,
  .analysis-body {
    grid-template-columns: 1fr;
  }
}

/* Legacy opensource reading exam shell: compact header, split panes, bottom question nav. */
.reading-page {
  --atlas-reading-nav-height: 72px;
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  gap: 0;
  height: 100vh;
  min-height: 100vh;
  overflow: hidden;
  padding-bottom: var(--atlas-reading-nav-height);
  background: var(--atlas-glass-pressed);
  color: var(--atlas-ink);
  font-family: var(--font-family-base);
}

.reading-page *,
.reading-page *::before,
.reading-page *::after {
  box-sizing: border-box;
}

.reading-header.header {
  position: relative;
  flex: 0 0 auto;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  min-height: 67px;
  padding: 14px 22px;
  border-bottom: 1px solid var(--atlas-line);
  background: var(--atlas-glass-elevated);
  box-shadow: 0 1px 2px rgba(15, 23, 42, 0.04);
}

.header-content {
  display: flex;
  min-width: 0;
  flex-direction: column;
  gap: 4px;
}

.reading-header h1 {
  margin: 0;
  color: var(--atlas-ink);
  font-family: inherit;
  font-size: 1.1rem;
  font-weight: 700;
  line-height: 1.4;
}

.reading-header p {
  max-width: none;
  margin: 0;
  color: var(--atlas-ink-soft);
  font-size: 0.92rem;
}

#review-nav-bar {
  position: absolute;
  left: 50%;
  top: 50%;
  z-index: 2;
  display: inline-flex;
  align-items: center;
  gap: 8px;
  transform: translate(-50%, -50%);
}

#review-nav-bar button {
  border: 1px solid rgba(148, 163, 184, 0.6);
  border-radius: 6px;
  padding: 4px 10px;
  background: var(--atlas-rim);
  color: var(--atlas-ink);
  cursor: pointer;
  font-size: 12px;
  font-weight: 600;
}

#review-nav-bar button:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.header-controls,
.reading-header-actions,
.answer-actions {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 10px;
}

.reading-stat,
.header-btn,
.practice-nav .controls button,
.submit-btn {
  min-height: 36px;
  border: 1px solid var(--atlas-line);
  border-radius: 4px;
  background: var(--atlas-glass-elevated);
  color: var(--atlas-ink);
  cursor: pointer;
  font: inherit;
  font-size: 0.92rem;
  line-height: 1.2;
  padding: 8px 12px;
  text-decoration: none;
}

.reading-stat {
  cursor: default;
  font-weight: 700;
}

.reading-timer:not(:disabled) {
  cursor: pointer;
  transition: opacity 0.2s ease, background 0.2s ease;
}

.reading-timer:not(:disabled):hover {
  opacity: 0.85;
}

.reading-timer.paused {
  opacity: 0.5;
}

.reading-timer:disabled {
  cursor: default;
}

.header-btn:hover:not(:disabled),
.practice-nav .controls button:hover:not(:disabled) {
  background: var(--atlas-glass-pressed);
}

.header-btn:disabled,
.practice-nav .controls button:disabled,
.submit-btn:disabled {
  cursor: default;
  opacity: 0.56;
}

.submit-btn {
  border-color: var(--atlas-accent);
  background: var(--atlas-accent);
  color: var(--atlas-rim);
  font-weight: 600;
}

.overlay {
  position: fixed;
  inset: 0;
  z-index: 2500;
  background: rgba(15, 23, 42, 0.18);
}

.reading-floating-panel {
  position: fixed;
  z-index: 2600;
  border: 1px solid var(--atlas-line);
  border-radius: 8px;
  background: var(--atlas-rim);
  color: var(--atlas-ink);
  box-shadow: 0 18px 48px rgba(15, 23, 42, 0.18);
}

.reading-settings-panel {
  top: 60px;
  right: 20px;
  display: block;
  width: min(260px, calc(100vw - 28px));
  padding: 16px;
  box-shadow: 0 10px 25px rgba(0, 0, 0, 0.1);
}

.reading-settings-panel .settings-section {
  margin-bottom: 16px;
}

.reading-settings-panel .settings-section:last-child {
  margin-bottom: 0;
}

.settings-title {
  color: var(--atlas-ink-soft);
  font-size: 0.9rem;
  font-weight: 600;
  margin: 0 0 8px;
}

.settings-options {
  display: flex;
  gap: 8px;
}

.settings-option {
  flex: 1;
  border: 1px solid var(--atlas-line);
  border-radius: 4px;
  background: var(--atlas-glass-elevated-alt);
  color: var(--atlas-ink);
  cursor: pointer;
  font: inherit;
  font-size: 0.86rem;
  padding: 8px;
  text-align: center;
}

.settings-option.active {
  border-color: var(--atlas-accent);
  background: var(--atlas-accent);
  color: var(--atlas-rim);
}

.reading-notes-panel {
  top: 50%;
  left: 50%;
  display: flex;
  width: min(400px, calc(100vw - 28px));
  height: min(300px, calc(100vh - 120px));
  max-height: min(300px, calc(100vh - 120px));
  flex-direction: column;
  padding: 0;
  transform: translate(-50%, -50%);
}

.reading-notes-panel header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 12px 16px;
  border-bottom: 1px solid var(--atlas-line);
  font-weight: 600;
}

.reading-notes-panel h3 {
  margin: 0;
  font-size: 1rem;
}

.reading-notes-panel textarea {
  flex: 1;
  min-height: 0;
  padding: 16px;
  border: none;
  background: transparent;
  color: var(--atlas-ink);
  font-family: inherit;
  resize: none;
}

.reading-notes-panel textarea:focus {
  outline: none;
}

.reading-selection-toolbar {
  position: absolute;
  z-index: 1000;
  display: flex;
  gap: 4px;
  padding: 4px;
  border-radius: 6px;
  background: var(--atlas-ink);
  color: white;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.25);
}

.reading-selection-toolbar button {
  border: 0;
  border-radius: 4px;
  background: transparent;
  color: white;
  cursor: pointer;
  font: inherit;
  font-size: 0.85rem;
  padding: 6px 12px;
}

.reading-selection-toolbar button:hover {
  background: var(--atlas-ink-soft);
}

.reading-html .hl,
.review-dictionary-highlight {
  background-color: var(--atlas-accent-strong);
  color: var(--atlas-rim);
  cursor: pointer;
}

.reading-html .hl[data-hl-type="note"] {
  background-color: var(--atlas-accent-alt);
  color: var(--atlas-rim);
}

.reading-html .memorize-locator-highlight {
  border-radius: 3px;
  background: var(--atlas-accent-soft);
  box-shadow: inset 0 -2px 0 var(--atlas-success);
}

.review-highlight-dictionary-bubble {
  position: fixed;
  z-index: 2800;
  width: min(340px, calc(100vw - 24px));
  max-height: min(420px, calc(100vh - 24px));
  overflow: auto;
  padding: 12px;
  border: 1px solid rgba(148, 163, 184, 0.45);
  border-radius: 8px;
  background: var(--atlas-rim);
  box-shadow: 0 18px 48px rgba(15, 23, 42, 0.22);
}

.vocab-bubble-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 10px;
  margin-bottom: 8px;
}

.vocab-term {
  margin: 0;
  overflow-wrap: anywhere;
  font-size: 1rem;
  font-weight: 700;
}

.vocab-part {
  border-top: 1px solid rgba(148, 163, 184, 0.25);
  padding-top: 7px;
  margin-top: 7px;
}

.vocab-part:first-of-type {
  border-top: 0;
  padding-top: 0;
}

.vocab-part-term {
  font-size: 0.94rem;
}

.vocab-meta,
.vocab-label {
  color: var(--atlas-ink-soft);
  font-size: 0.78rem;
}

.vocab-meta {
  margin-top: 2px;
  overflow-wrap: anywhere;
}

.vocab-label {
  font-weight: 700;
}

.vocab-section {
  margin-top: 8px;
}

.vocab-text {
  color: var(--atlas-ink-soft);
  font-size: 0.88rem;
  line-height: 1.5;
  white-space: pre-wrap;
}

.vocab-actions {
  display: flex;
  justify-content: flex-end;
  margin-top: 12px;
}

.vocab-close,
.vocab-add {
  border: 1px solid var(--atlas-line);
  border-radius: 6px;
  background: var(--atlas-rim);
  color: var(--atlas-ink);
  cursor: pointer;
  font: inherit;
  padding: 6px 10px;
}

.vocab-add {
  border-color: var(--atlas-accent);
  background: var(--atlas-accent);
  color: var(--atlas-rim);
  font-weight: 700;
}

.vocab-add:disabled {
  cursor: default;
  opacity: 0.78;
}

.reading-workspace.shell {
  --atlas-reading-left-pane-width: clamp(360px, 50%, calc(100% - 360px));
  --atlas-reading-divider-width: 10px;
  flex: 1 1 auto;
  display: grid;
  grid-template-columns:
    minmax(280px, var(--atlas-reading-left-pane-width))
    var(--atlas-reading-divider-width)
    minmax(320px, 1fr);
  grid-template-rows: minmax(0, 1fr);
  align-items: stretch;
  width: 100%;
  min-height: 0;
  max-height: 100%;
  height: auto;
  overflow: hidden;
}

.reading-settings-panel .settings-panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 16px;
}

.reading-settings-panel .settings-panel-header h2 {
  margin: 0;
  font: inherit;
  font-weight: 700;
}

.reading-settings-panel .settings-panel-close {
  flex: 0 0 auto;
}

.pane {
  min-height: 0;
  height: 100%;
  overflow: auto;
  overscroll-behavior: contain;
  background: var(--atlas-glass-elevated);
  padding: 22px;
  min-width: 0;
}

#left {
  min-width: 0;
}

#reading-divider {
  align-self: stretch;
  height: 100%;
  min-height: 100%;
  border-right: 1px solid var(--atlas-line);
  border-left: 1px solid var(--atlas-line);
  background: var(--atlas-glass-pressed);
  cursor: ew-resize;
  outline: none;
  position: relative;
  touch-action: none;
  user-select: none;
}

#reading-divider::before {
  content: "";
  position: absolute;
  top: 50%;
  left: 50%;
  width: 2px;
  height: 48px;
  border-radius: 999px;
  background: rgba(100, 116, 139, 0.55);
  box-shadow: -3px 0 0 rgba(100, 116, 139, 0.28), 3px 0 0 rgba(100, 116, 139, 0.28);
  transform: translate(-50%, -50%);
}

#reading-divider:hover,
#reading-divider:focus-visible,
#reading-divider.is-dragging {
  border-color: var(--atlas-accent-alt);
  background: var(--atlas-accent);
}

#right {
  min-width: 0;
  background: var(--atlas-glass-elevated-alt);
  padding: 12px 14px;
}

.reading-html {
  color: var(--atlas-ink);
}

.reading-html h2,
.reading-html h3,
.reading-html h4,
.reading-html h5 {
  margin: 18px 0 10px;
  color: var(--atlas-ink);
  font-family: inherit;
}

.reading-html p,
#left p,
#right p {
  margin: 0 0 14px;
  line-height: 1.7;
}

.reading-html label {
  display: block;
  margin: 8px 0;
  line-height: 1.55;
}

.reading-html .choice-item label,
.reading-html .checkbox-options label,
.reading-html .options-list label,
.reading-html .multiple-choice-options label,
.reading-html .matching-options label,
.reading-html .mcq-group label,
.reading-html .radio-options label,
.reading-html .radio-group label,
.reading-html .multiple-choice label,
.reading-html .true-false-ng label {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  margin-bottom: 12px;
  cursor: pointer;
  line-height: 1.5;
}

.reading-html .choice-item label input[type="checkbox"],
.reading-html .checkbox-options label input[type="checkbox"],
.reading-html .options-list label input[type="checkbox"],
.reading-html .choice-item label input[type="radio"],
.reading-html .multiple-choice-options label input[type="radio"],
.reading-html .matching-options label input[type="radio"],
.reading-html .mcq-group label input[type="radio"],
.reading-html .radio-options label input[type="radio"],
.reading-html .radio-group label input[type="radio"],
.reading-html .multiple-choice label input[type="radio"],
.reading-html .true-false-ng label input[type="radio"] {
  flex-shrink: 0;
  margin-top: 4px;
  cursor: pointer;
}

.reading-html input[type="text"],
.reading-html textarea,
.reading-html select,
.practice-nav select,
.practice-nav input[type="text"] {
  border: 1px solid var(--atlas-line);
  border-radius: 8px;
  background: var(--atlas-rim);
  color: var(--atlas-ink);
  font: inherit;
  min-height: 32px;
  padding: 6px 8px;
}

.reading-html input[type="radio"],
.reading-html input[type="checkbox"],
.practice-nav input[type="checkbox"] {
  width: 18px;
  height: 18px;
  accent-color: var(--atlas-accent);
  vertical-align: middle;
}

.reading-html table {
  width: 100%;
  margin: 14px 0;
  border-collapse: collapse;
  background: var(--atlas-rim);
}

.reading-html th,
.reading-html td {
  padding: 8px 10px;
  border: 1px solid var(--atlas-line);
  vertical-align: top;
}

.reading-html .table-section {
  margin-top: 12px;
  overflow-x: auto;
  padding: 10px 12px;
  border: 1px solid var(--atlas-line);
  border-radius: 8px;
  background: var(--atlas-glass-elevated);
}

.reading-html .table-section table {
  width: 100%;
  margin: 0;
  table-layout: fixed;
  border-collapse: collapse;
}

.reading-html .table-section thead th {
  padding: 10px 8px;
  border: 1px solid var(--atlas-line);
  background: var(--atlas-glass-elevated-alt);
  text-align: center;
  font-weight: 700;
}

.reading-html .table-section tbody td {
  padding: 10px 10px;
  border: 1px solid var(--atlas-line);
  vertical-align: top;
  line-height: 1.45;
}

.reading-html .table-section thead th:first-child,
.reading-html .table-section tbody td:first-child {
  width: 16%;
}

.reading-html .table-section thead th:nth-child(2),
.reading-html .table-section tbody td:nth-child(2) {
  width: 20%;
}

.reading-html .table-section thead th:nth-child(3),
.reading-html .table-section tbody td:nth-child(3) {
  width: 64%;
}

.reading-html .table-section ul {
  margin: 0;
  padding-left: 20px;
}

.reading-html .table-section li + li {
  margin-top: 6px;
}

.reading-html .table-section input.blank {
  display: inline-block;
  min-width: 120px;
  max-width: 180px;
  width: 40%;
  padding: 2px 6px;
  border: 1px solid var(--atlas-line);
  border-radius: 4px;
  background: var(--atlas-glass-elevated);
  color: var(--atlas-ink);
  vertical-align: middle;
}

.unified-group.question-group {
  margin-bottom: 16px;
  padding: 0;
  border: 0;
  border-radius: 4px;
}

.unified-group__lead {
  margin-bottom: 12px;
}

.reading-html .group {
  margin-bottom: 0;
  padding: 18px 22px;
  border: 1px solid var(--atlas-line);
  border-radius: 4px;
  background: var(--atlas-glass-elevated);
  box-shadow: var(--atlas-shadow);
}

#results.review-panel {
  margin: 18px 0 0;
  padding: 18px;
  border: 1px solid var(--atlas-line);
  border-radius: 4px;
  background: var(--atlas-glass-elevated);
  box-shadow: var(--atlas-shadow);
}

.reading-explanation-panel {
  display: grid;
  gap: 10px;
  margin: 14px 0 22px;
}

.reading-explanation-card {
  margin: 10px 0 14px;
  padding: 10px 12px;
  border: 1px solid rgba(37, 99, 235, 0.22);
  border-left: 4px solid rgba(37, 99, 235, 0.9);
  border-radius: 8px;
  background: rgba(239, 246, 255, 0.75);
}

.reading-explanation-card__label {
  margin-bottom: 6px;
  color: var(--atlas-accent-strong);
  font-size: 12px;
  font-weight: 700;
  line-height: 1.3;
}

.reading-explanation-card__text {
  color: var(--atlas-ink);
  font-size: 14px;
  line-height: 1.6;
  white-space: pre-wrap;
}

.reading-group-explanation,
.reading-question-explanation {
  margin-top: 10px;
}

.reading-question-explanation-list {
  margin-top: 10px;
  padding: 10px 12px;
  border: 1px solid rgba(37, 99, 235, 0.16);
  border-radius: 8px;
  background: var(--atlas-glass);
}

.reading-question-explanation-list h5 {
  margin: 0 0 8px;
  color: var(--atlas-accent-strong);
  font-size: 13px;
  font-weight: 700;
}

.reading-question-explanation-item {
  margin: 8px 0;
}

.reading-html .question-item,
.reading-html .match-question-item,
.reading-html .choice-item {
  margin-bottom: 12px;
}

.reading-html .paragraph-dropzone,
.reading-html .match-dropzone,
.dragdrop-slot {
  display: flex;
  min-height: 40px;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
  margin: 0 0 8px;
  padding: 8px 10px;
  border: 2px dashed var(--atlas-accent-ring);
  border-radius: 4px;
  background: var(--atlas-accent-soft);
  color: var(--atlas-ink-soft);
  transition: border-color 0.16s ease, background 0.16s ease;
}

.reading-html .paragraph-dropzone.drag-over,
.reading-html .match-dropzone.drag-over,
.reading-html .drag-over,
.dragdrop-slot.drag-over {
  border-color: var(--atlas-accent);
  background: var(--atlas-accent-soft);
  box-shadow: none;
}

.reading-html .paragraph-dropzone.dropzone-filled,
.reading-html .match-dropzone.dropzone-filled,
.dragdrop-slot.filled {
  border-style: solid;
}

.reading-html .drop-target-summary {
  position: relative;
  display: inline-flex;
  min-width: 80px;
  min-height: 30px;
  align-items: center;
  justify-content: center;
  margin: 0 4px;
  padding: 0 8px;
  border: 0;
  border-bottom: 2px solid var(--atlas-ink-soft);
  border-radius: 4px 4px 0 0;
  background: rgba(0, 0, 0, 0.02);
  box-shadow: none;
  vertical-align: bottom;
  transition: all 0.2s ease;
}

.reading-html .drop-target-summary.drag-over {
  border-bottom-color: var(--atlas-accent);
  background: var(--atlas-accent-soft);
}

.reading-html .drop-target-summary.dropzone-filled {
  border-bottom-color: var(--atlas-success);
  background: var(--atlas-accent-soft);
  color: var(--atlas-success);
  font-weight: 600;
}

.reading-html .drop-target-summary .drag-item,
.reading-html .drop-target-summary .dragdrop-chip {
  margin: 0;
  padding: 0;
  border: 0;
  background: transparent;
  color: inherit;
  cursor: grab;
  font-weight: inherit;
}

.reading-html .paragraph-label {
  color: var(--atlas-ink-soft);
  font-size: 0.9rem;
  font-weight: 700;
  white-space: nowrap;
}

.reading-html .pool-items,
.reading-html .options-pool,
.reading-html .headings-pool,
.reading-html .dropped-items,
.dragdrop-options {
  display: flex;
  min-height: 40px;
  flex: 1;
  flex-wrap: wrap;
  gap: 8px;
}

.reading-html .pool-items.drag-over,
.reading-html .options-pool.drag-over,
.reading-html .headings-pool.drag-over {
  border-color: var(--atlas-accent);
  background: var(--atlas-accent-soft);
  box-shadow: none;
}

.reading-html .dropped-items:empty::after {
  content: "拖到这里";
  color: var(--atlas-ink-faint);
  font-size: 0.82rem;
}

.reading-html .drag-item,
.dragdrop-chip,
.reading-html .dragdrop-chip {
  min-height: 32px;
  max-width: 100%;
  padding: 6px 10px;
  border: 1px solid var(--atlas-accent-ring);
  border-radius: 4px;
  background: var(--atlas-glass-pressed);
  color: var(--atlas-ink);
  cursor: grab;
  font: inherit;
  font-size: 0.9rem;
  line-height: 1.35;
  overflow-wrap: anywhere;
  text-align: left;
  user-select: none;
}

.reading-html .drag-item.dragging,
.dragdrop-chip.dragging,
.reading-html .dragging {
  opacity: 0.55;
}

.reading-html .drag-item--assigned,
.dragdrop-chip-assigned,
.reading-html .dragdrop-chip-assigned {
  border-style: solid;
  border-color: var(--atlas-accent-ring);
  background: var(--atlas-glass-pressed);
}

.dragdrop-option.selected {
  border-color: var(--atlas-accent-ring);
  background: var(--atlas-accent-soft);
}

.dragdrop-option:disabled:not(.selected) {
  cursor: not-allowed;
  opacity: 0.38;
}

.practice-nav {
  position: fixed;
  top: auto;
  right: 0;
  bottom: 0;
  left: 0;
  z-index: 2000;
  display: flex;
  align-items: center;
  gap: 16px;
  height: var(--atlas-reading-nav-height);
  min-height: var(--atlas-reading-nav-height);
  padding: 12px 18px;
  border-top: 1px solid var(--atlas-line);
  background: var(--atlas-glass-elevated);
  box-shadow: 0 -8px 24px rgba(15, 23, 42, 0.06);
  overflow: visible;
}

.practice-nav .title {
  flex: 0 0 auto;
  color: var(--atlas-ink-soft);
  font-weight: 700;
  white-space: nowrap;
}

.practice-nav .questions {
  display: flex;
  max-height: none;
  flex: 1 1 auto;
  flex-wrap: nowrap;
  gap: 8px;
  overflow-x: auto;
  overflow-y: visible;
  padding: 0;
}

.practice-nav .question-nav-entry {
  position: relative;
  display: inline-flex;
  flex: 0 0 auto;
  align-items: center;
}

.practice-nav .q-item {
  position: relative;
  display: inline-flex;
  flex: 0 0 auto;
  align-items: center;
  justify-content: center;
  padding: 6px 12px;
  border: 1px solid var(--atlas-line);
  border-radius: 4px;
  background: var(--atlas-glass-elevated);
  color: var(--atlas-ink);
  cursor: pointer;
  font-size: 0.9rem;
}

.practice-nav .controls {
  display: flex;
  align-items: center;
  flex: 0 0 auto;
  flex-wrap: nowrap;
  gap: 8px;
  margin-left: auto;
}

.practice-nav .q-item:hover {
  background: var(--atlas-glass-pressed);
}

.practice-nav .q-item.answered {
  border-color: var(--atlas-accent-ring);
  background: var(--atlas-accent-soft);
}

.practice-nav .q-item.active,
.practice-nav .q-item.answered.active {
  border-color: var(--atlas-accent);
  background: var(--atlas-accent-soft);
  color: var(--atlas-accent);
  font-weight: 600;
}

.practice-nav .q-item.correct,
.practice-nav .q-item.review-correct {
  border-color: var(--atlas-success);
  background: var(--atlas-accent-soft);
  color: var(--atlas-success);
}

.practice-nav .q-item.incorrect,
.practice-nav .q-item.review-incorrect {
  border-color: var(--atlas-danger);
  background: var(--atlas-library-danger);
  color: var(--atlas-danger);
}

.practice-nav .q-item.marked::after {
  position: absolute;
  top: -5px;
  right: -5px;
  width: 10px;
  height: 10px;
  border: 2px solid var(--atlas-rim);
  border-radius: 999px;
  background: var(--atlas-warning);
  content: "";
}

.mark-question-button {
  min-width: 18px;
  min-height: 18px;
  margin-left: -4px;
  padding: 0 4px;
  border: 1px solid rgba(100, 116, 139, 0.45);
  border-radius: 6px;
  background: var(--atlas-glass);
  color: var(--atlas-ink-soft);
  cursor: pointer;
  font: inherit;
  font-size: 0.72rem;
  font-weight: 800;
  line-height: 1;
}

.mark-question-button.active {
  border-color: var(--atlas-warning);
  background: var(--atlas-accent-soft);
  color: var(--atlas-accent-strong);
}

.mark-question-button:disabled {
  cursor: default;
  opacity: 0.62;
}

.dragdrop-select {
  width: 100%;
}

.suite-progress-mini {
  display: grid;
  min-height: 36px;
  align-items: center;
  padding: 4px 10px;
  border: 1px solid var(--atlas-accent-ring);
  border-radius: 8px;
  background: var(--atlas-accent-soft);
}

.suite-progress-mini span {
  display: block;
  color: var(--atlas-ink-soft);
  font-size: 0.72rem;
}

.suite-progress-mini strong {
  display: block;
  color: var(--atlas-ink);
  font-size: 0.86rem;
}

.reading-action-status,
.snapshot-message {
  position: static;
  flex: 0 0 auto;
  align-self: center;
  width: min(42rem, calc(100% - 36px));
  margin: 8px 18px;
  padding: 8px 10px;
  border: 1px solid var(--atlas-rim);
  border-radius: 8px;
  background: var(--atlas-glass-elevated);
  color: var(--atlas-ink);
  font-size: 0.86rem;
  box-shadow: var(--atlas-shadow);
}

.panel-kicker {
  color: var(--atlas-accent);
  font-size: 0.76rem;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.panel-heading,
.answer-panel-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 18px;
}

.score-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 10px;
  margin-bottom: 18px;
}

.score-grid > div,
.analysis-strip > div {
  padding: 10px 12px;
  border: 1px solid var(--atlas-line);
  border-radius: 8px;
  background: var(--atlas-glass-pressed);
}

.score-grid span,
.analysis-strip span {
  display: block;
  color: var(--atlas-ink-soft);
  font-size: 0.75rem;
}

.score-grid strong,
.analysis-strip strong {
  display: block;
  margin-top: 3px;
  color: var(--atlas-ink);
}

.review-analysis {
  display: grid;
  gap: 16px;
  margin: 4px 0 20px;
  padding: 16px 0 2px;
  border-top: 1px solid var(--atlas-line);
  border-bottom: 1px solid var(--atlas-line);
}

.llm-review-status {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 10px;
}

.llm-review-status > strong {
  min-width: 0;
  flex: 1;
}

.analysis-strip,
.analysis-body {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
}

.llm-question-analysis-list {
  grid-column: 1 / -1;
  display: grid;
  gap: 10px;
}

.llm-question-analysis {
  display: grid;
  grid-template-columns: 52px minmax(0, 1fr);
  gap: 10px;
  padding: 10px 0;
  border-top: 1px solid var(--atlas-line);
  color: var(--atlas-ink-soft);
  font-size: 0.86rem;
}

.llm-question-analysis > strong {
  color: var(--atlas-ink);
}

.llm-question-analysis dl {
  display: grid;
  grid-template-columns: 72px minmax(0, 1fr);
  gap: 6px 10px;
  margin: 0;
}

.llm-question-analysis dt {
  color: var(--atlas-ink-soft);
}

.llm-question-analysis dd {
  min-width: 0;
  margin: 0;
  overflow-wrap: anywhere;
}

.analysis-list {
  display: grid;
  gap: 8px;
  margin: 0;
  padding: 0;
  list-style: none;
}

.analysis-list li {
  display: grid;
  grid-template-columns: 42px minmax(0, 1fr);
  gap: 8px;
  color: var(--atlas-ink-soft);
  font-size: 0.88rem;
}

.kind-bar-row {
  display: grid;
  grid-template-columns: minmax(92px, 0.24fr) minmax(0, 1fr) 54px;
  gap: 10px;
  align-items: center;
  color: var(--atlas-ink-soft);
  font-size: 0.86rem;
}

.kind-bar-track {
  height: 8px;
  overflow: hidden;
  border-radius: 999px;
  background: var(--atlas-glass-pressed);
}

.kind-bar-track i {
  display: block;
  height: 100%;
  border-radius: inherit;
  background: var(--atlas-success);
}

.review-table,
.results-table {
  width: 100%;
  border-collapse: collapse;
  margin-top: 12px;
  overflow-wrap: anywhere;
  font-size: 0.92rem;
}

.review-table th,
.review-table td,
.results-table th,
.results-table td {
  padding: 8px 10px;
  border: 1px solid var(--atlas-line);
  text-align: center;
  vertical-align: top;
}

.review-correct td:last-child,
.result-correct {
  color: var(--atlas-success);
  font-weight: 700;
}

.review-incorrect td:last-child,
.result-incorrect {
  color: var(--atlas-danger);
  font-weight: 700;
}

.reading-coach-fab {
  position: fixed;
  right: 16px;
  bottom: 20px;
  z-index: 9999;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 10px 14px;
  border: 0;
  border-radius: 999px;
  background: var(--atlas-accent);
  color: var(--atlas-rim);
  box-shadow: var(--atlas-shadow-high);
  cursor: pointer;
  font: inherit;
  font-size: 13px;
  font-weight: 700;
  touch-action: none;
  user-select: none;
}

.reading-coach-panel {
  position: fixed;
  right: 16px;
  bottom: 70px;
  z-index: 9999;
  display: none;
  overflow: hidden;
  width: min(380px, calc(100vw - 24px));
  max-height: min(68vh, 560px);
  border: 1px solid var(--atlas-rim);
  border-radius: var(--atlas-radius-sm);
  background: var(--atlas-glass-elevated);
  box-shadow: var(--atlas-shadow-high);
  touch-action: none;
}

.reading-coach-panel.is-open {
  display: flex;
  flex-direction: column;
}

.reading-coach-panel__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 12px;
  border-bottom: 1px solid var(--atlas-line);
  background: var(--atlas-glass-pressed);
}

.reading-coach-panel__title {
  color: var(--atlas-ink);
  font-size: 13px;
  font-weight: 700;
}

.reading-coach-panel__close {
  border: 0;
  background: transparent;
  color: var(--atlas-ink-soft);
  cursor: pointer;
  font-size: 16px;
}

.reading-coach-panel__status,
.reading-coach-panel__error {
  min-height: 22px;
  padding: 8px 12px 0;
  color: var(--atlas-accent-alt);
  font-size: 12px;
}

.reading-coach-panel__status.is-error,
.reading-coach-panel__error {
  color: var(--atlas-danger);
}

.reading-coach-panel__messages {
  display: grid;
  flex: 1;
  gap: 8px;
  overflow: auto;
  padding: 10px 12px;
  background: var(--atlas-glass-pressed);
}

.reading-coach-msg {
  padding: 8px 10px;
  border-radius: 10px;
  white-space: pre-wrap;
  font-size: 12px;
  line-height: 1.5;
  overflow-wrap: anywhere;
}

.reading-coach-msg.user {
  justify-self: end;
  background: var(--atlas-accent-soft);
  color: var(--atlas-accent-strong);
}

.reading-coach-msg.assistant {
  justify-self: start;
  border: 1px solid var(--atlas-line);
  background: var(--atlas-glass-elevated);
  color: var(--atlas-ink);
}

.reading-coach-msg.error {
  border-color: var(--atlas-danger);
  color: var(--atlas-danger);
}

.reading-coach-panel__actions,
.reading-coach-panel__followups {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  padding: 8px 12px 0;
}

.reading-coach-chip {
  padding: 4px 8px;
  border: 1px solid var(--atlas-accent-ring);
  border-radius: 999px;
  background: var(--atlas-glass-elevated);
  color: var(--atlas-accent-strong);
  cursor: pointer;
  font: inherit;
  font-size: 11px;
  font-weight: 600;
}

.reading-coach-chip:disabled {
  cursor: not-allowed;
  opacity: 0.55;
}

.reading-coach-panel__selected-context {
  display: grid;
  gap: 6px;
  padding: 8px 12px 0;
  color: var(--atlas-ink-soft);
  font-size: 12px;
}

.reading-coach-panel__selected-context span {
  color: var(--atlas-ink-faint);
  font-size: 11px;
  font-weight: 700;
}

.reading-coach-panel__selected-context strong {
  max-height: 4.8em;
  overflow: hidden;
  font-weight: 600;
  line-height: 1.5;
}

.reading-coach-panel__composer {
  display: flex;
  gap: 8px;
  padding: 10px 12px 12px;
  border-top: 1px solid var(--atlas-line);
  background: var(--atlas-glass-elevated);
}

.reading-coach-panel__input {
  flex: 1;
  min-width: 0;
  padding: 8px;
  border: 1px solid var(--atlas-line);
  border-radius: var(--atlas-radius-sm);
  background: var(--atlas-glass-pressed);
  color: var(--atlas-ink);
  font: inherit;
  font-size: 12px;
}

.reading-coach-panel__send {
  padding: 0 12px;
  border: 0;
  border-radius: var(--atlas-radius-sm);
  background: var(--atlas-accent);
  color: var(--atlas-rim);
  cursor: pointer;
  font: inherit;
  font-size: 12px;
  font-weight: 700;
}

.reading-coach-panel__send:disabled {
  cursor: not-allowed;
  opacity: 0.55;
}

@media (max-width: 980px) {
  .reading-coach-panel {
    right: 8px;
    left: 8px;
    bottom: 64px;
    width: auto;
    max-height: 70vh;
  }

  .reading-coach-fab {
    right: 10px;
    bottom: 12px;
  }

  .reading-page {
    --atlas-reading-nav-height: 112px;
    min-height: 100vh;
    height: auto;
    overflow: auto;
    padding-bottom: var(--atlas-reading-nav-height);
  }

  .reading-header.header {
    align-items: flex-start;
    flex-direction: column;
  }

  .reading-workspace.shell {
    display: block;
    height: auto;
    max-height: none;
    overflow: visible;
    min-height: 0;
  }

  .reading-workspace.shell .pane {
    min-height: auto;
    height: auto;
    overflow: visible;
  }

  #left,
  #right {
    width: 100%;
    min-width: 0;
    flex: none;
  }

  #reading-divider {
    display: none;
  }

  .practice-nav {
    align-items: flex-start;
    flex-wrap: wrap;
    gap: 10px;
    height: var(--atlas-reading-nav-height);
    overflow-y: auto;
  }

  .practice-nav .questions {
    order: 3;
    flex-basis: 100%;
    flex-wrap: nowrap;
    overflow-x: auto;
  }

  .practice-nav .controls {
    margin-left: 0;
    width: 100%;
    justify-content: flex-end;
  }

  .reading-html .table-section input.blank {
    width: 100%;
    max-width: none;
    margin-top: 4px;
  }

  .score-grid,
  .analysis-strip,
  .analysis-body {
    grid-template-columns: 1fr;
  }
}

.drag-option-selected {
  outline: 3px solid var(--atlas-accent);
  outline-offset: 2px;
  box-shadow: 0 0 0 4px rgb(37 99 235 / 18%);
}
</style>
