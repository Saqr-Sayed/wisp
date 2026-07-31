<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import AnalysisTab from './components/AnalysisTab.vue'
import Timeline from './components/Timeline.vue'
import SettingsPage from './components/SettingsPage.vue'
import { getTimeline, getLimits, periodRange, eventDuration, categoryLabel, categoryColor, type Period, type LogEntry } from './lib/dbus'

const view = ref<'dashboard' | 'settings'>('dashboard')
const period = ref<Period>('day')
const offset = ref(0)
const logs = ref<LogEntry[]>([])
const todayLogs = ref<LogEntry[]>([])
const limits = ref<[string, string, number][]>([])
const nowSec = ref(Math.floor(Date.now() / 1000))
const loading = ref(true)
const error = ref(false)
const groupBy = ref<'app' | 'category' | 'site' | 'series'>('app')
const searchQuery = ref('')

async function refresh() {
  try {
    nowSec.value = Math.floor(Date.now() / 1000)
    const [from, to] = periodRange(period.value, offset.value)
    logs.value = await getTimeline(from, to)
    const [tFrom, tTo] = periodRange('day', 0)
    todayLogs.value = await getTimeline(tFrom, tTo)
    limits.value = await getLimits()
    evaluateLimits()
    error.value = false
  } catch {
    error.value = true
  } finally {
    loading.value = false
  }
}

const overSet = ref(new Map<string, string>()) // key "kind:target" -> رسالة
let lastNotified = new Map<string, number>() // key -> epoch of last notification

function evalLabel(target: string, kind: string): string {
  if (kind === 'category') return categoryLabel(target)
  const hit = todayLogs.value.find(l => l.app_name === target)
  return hit?.friendly_name ?? target
}

function evaluateLimits() {
  const used = new Map<string, number>()
  for (const l of todayLogs.value) {
    const d = eventDuration(l, nowSec.value)
    used.set(`category:${l.category}`, (used.get(`category:${l.category}`) ?? 0) + d)
    used.set(`app:${l.app_name}`, (used.get(`app:${l.app_name}`) ?? 0) + d)
  }
  const over = new Map<string, string>()
  for (const [target, kind, minutes] of limits.value) {
    const u = (used.get(`${kind}:${target}`) ?? 0) / 60
    if (u > minutes) over.set(`${kind}:${target}`, `تجاوزت حد ${evalLabel(target, kind)}: ${minutes} دقيقة`)
  }
  overSet.value = over
}

function notify() {
  const nowMs = Date.now()
  for (const [key, msg] of overSet.value) {
    const last = lastNotified.get(key) ?? 0
    if (nowMs - last > 10 * 60 * 1000) { // تذكير كل 10 دقائق
      invoke('notify', { body: msg }).then(() => lastNotified.set(key, nowMs)).catch(() => {})
    }
  }
}

let timer: number | undefined
onMounted(async () => {
  await refresh()
  timer = window.setInterval(async () => { await refresh(); notify() }, 5000)
})
onUnmounted(() => window.clearInterval(timer))

const todayLabel = computed(() => {
  const p = period.value
  const name = p === 'day' ? 'اليوم' : p === 'week' ? 'الأسبوع' : 'الشهر'
  return offset.value === 0 ? name : `${name} الماضي (${offset.value})`
})

const periodTotal = computed(() => logs.value.reduce((s, l) => s + eventDuration(l), 0))
</script>

<template>
  <div id="shell" class="rtl">
    <header class="hdr">
      <div class="hdr-row">
        <div class="brand">سالمونيلا</div>
        <div class="period">
          <button class="btn ghost sq" @click="offset++; refresh()" aria-label="الفترة التالية">‹</button>
          <span class="period-label">{{ todayLabel }}</span>
          <button class="btn ghost sq" @click="offset = Math.max(0, offset - 1); refresh()" aria-label="الفترة السابقة">›</button>
          <select :value="period" @change="period = ($event.target as HTMLSelectElement).value as Period; offset = 0; refresh()">
            <option value="day">اليوم</option>
            <option value="week">الأسبوع</option>
            <option value="month">الشهر</option>
          </select>
        </div>
        <button v-if="view === 'dashboard'" class="icon-btn" aria-label="الإعدادات" @click="view = 'settings'">
          <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/></svg>
        </button>
      </div>

      <div v-if="error" class="error-banner" role="alert">
        تعذّر تحديث البيانات
        <button class="btn primary small" @click="refresh">إعادة المحاولة</button>
      </div>
      <div v-if="overSet.size" class="over-banner" role="alert">
        ⚠ {{ [...overSet.values()].join(' · ') }}
      </div>
    </header>

    <main>
      <Transition name="view" mode="out-in">
        <div v-if="view === 'dashboard'" key="dash" class="dash">
          <div class="ribbon card" v-if="periodTotal > 0" role="img" aria-label="شريط الفترة">
            <div v-for="l in logs" :key="'rib' + l.id" class="rib-seg"
              :style="{ flexGrow: Math.max(1, eventDuration(l)), background: categoryColor(l.category) }"
              :title="l.window_title"></div>
          </div>
          <div class="ribbon card" v-else></div>

          <div class="cols">
            <AnalysisTab class="a-col" :logs="logs" :period="period" :offset="offset" :loading="loading" v-model:groupBy="groupBy" />
            <Timeline class="t-col" :logs="logs" :loading="loading" v-model:query="searchQuery" />
          </div>
        </div>

        <SettingsPage v-else key="settings" :limits="limits" :todayLogs="todayLogs"
          @back="view = 'dashboard'" @changed="refresh" />
      </Transition>
    </main>
  </div>
</template>

<style scoped>
.hdr { padding: 0.9rem 0 0.5rem; }
.hdr-row { display: flex; align-items: center; gap: 0.9rem; }
.brand { font-size: 1.1rem; font-weight: 900; color: var(--accent); }
.period { display: flex; align-items: center; gap: 0.4rem; margin-right: auto; }
.btn.sq { padding: 0.35rem 0.7rem; border-radius: 10px; }
.period-label { font-weight: 700; font-size: 0.9rem; min-width: 5.5rem; text-align: center; }
.period select {
  background: var(--surface-soft); border: 1px solid var(--border); border-radius: 10px;
  padding: 0.35rem 0.6rem; color: var(--ink); font-family: inherit; font-size: 0.85rem;
}
.btn.small { padding: 0.25rem 0.7rem; font-size: 0.75rem; }
.error-banner {
  margin-top: 0.5rem; display: flex; align-items: center; gap: 0.8rem;
  background: var(--danger-soft); border: 1px solid var(--danger); border-radius: var(--radius-sm);
  padding: 0.45rem 0.9rem; color: var(--danger); font-size: 0.85rem; font-weight: 600;
  animation: banner-in 200ms ease;
}
.over-banner {
  margin-top: 0.5rem;
  background: var(--danger-soft); border: 1px solid var(--danger); border-radius: var(--radius-sm);
  padding: 0.45rem 0.9rem; color: var(--danger); font-size: 0.8rem; font-weight: 600;
  animation: banner-in 200ms ease;
}
@keyframes banner-in { from { opacity: 0; transform: translateY(-6px); } to { opacity: 1; transform: translateY(0); } }

.dash { flex: 1; min-height: 0; display: flex; flex-direction: column; gap: 14px; padding-bottom: 1rem; }
.ribbon { display: flex; gap: 3px; padding: 8px 10px; flex-shrink: 0; overflow: hidden; }
.rib-seg { height: 14px; border-radius: 4px; min-width: 3px; }
.cols { flex: 1; min-height: 0; display: flex; gap: 14px; }
.a-col { flex: 3; min-width: 0; }
.t-col { flex: 2; max-width: 480px; min-width: 0; }
</style>
