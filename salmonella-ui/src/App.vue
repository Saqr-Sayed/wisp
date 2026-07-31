<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import AnalysisTab from './components/AnalysisTab.vue'
import Timeline from './components/Timeline.vue'
import WeekNav from './components/WeekNav.vue'
import SettingsPage from './components/SettingsPage.vue'
import { getTimeline, getLimits, eventDuration, categoryLabel, type LogEntry } from './lib/dbus'
import { startOfDay, daysOfWeek, dayRange } from './lib/dates'

const view = ref<'dashboard' | 'settings'>('dashboard')
const weekOffset = ref(0)
const selectedDay = ref(startOfDay(new Date()))
const dayLogs = ref<LogEntry[]>([])
const weekLogs = ref<LogEntry[]>([])
const limits = ref<[string, string, number][]>([])
const nowSec = ref(Math.floor(Date.now() / 1000))
const loading = ref(true)
const error = ref(false)
const groupBy = ref<'app' | 'category' | 'site' | 'series'>('app')
const searchQuery = ref('')

async function refresh() {
  try {
    nowSec.value = Math.floor(Date.now() / 1000)
    const [dFrom, dTo] = dayRange(selectedDay.value)
    dayLogs.value = await getTimeline(dFrom, dTo)
    const [wFrom, wTo] = dayRange(weekDays.value[6])
    weekLogs.value = await getTimeline(wFrom, wTo)
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
  const hit = dayLogs.value.find(l => l.app_name === target)
  return hit?.friendly_name ?? target
}

function evaluateLimits() {
  const used = new Map<string, number>()
  for (const l of dayLogs.value) {
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

const weekDays = computed(() => daysOfWeek(weekOffset.value))
const dayRangeNow = computed(() => dayRange(selectedDay.value))

function selectDay(d: Date) {
  selectedDay.value = startOfDay(d)
  refresh()
}

const ribbonSegs = computed(() => {
  const m = new Map<string, number>()
  for (const l of dayLogs.value) {
    const cat = l.category || 'other'
    m.set(cat, (m.get(cat) ?? 0) + eventDuration(l))
  }
  return [...m.entries()].sort((a, b) => b[1] - a[1])
})
</script>

<template>
  <div id="shell" class="rtl">
    <header class="hdr">
      <div class="hdr-row">
        <div class="brand">سالمونيلا</div>
        <button v-if="view === 'dashboard'" class="icon-btn gear" aria-label="الإعدادات" @click="view = 'settings'">
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
          <WeekNav class="w-nav" :days="weekDays" :logs="weekLogs" :dayLogs="dayLogs" :selected="selectedDay" :limits="limits"
            @select="selectDay" @prev="weekOffset++; refresh()"
            @next="weekOffset = Math.max(0, weekOffset - 1); refresh()" />

          <div class="cols">
            <AnalysisTab class="a-col" :logs="dayLogs" :range="dayRangeNow" :loading="loading" v-model:groupBy="groupBy" />
            <Timeline class="t-col" :logs="dayLogs" :loading="loading" v-model:query="searchQuery" :ribbon="ribbonSegs" />
          </div>
        </div>

        <SettingsPage v-else key="settings" :limits="limits" :todayLogs="dayLogs"
          @back="view = 'dashboard'" @changed="refresh" />
      </Transition>
    </main>
  </div>
</template>

<style scoped>
.hdr { padding: 0.9rem 0 0.5rem; }
.hdr-row { display: flex; align-items: center; gap: 0.9rem; }
.hdr .gear { margin-inline-start: auto; }
.brand { font-size: 1.1rem; font-weight: 900; color: var(--accent); }
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
.cols { flex: 1; min-height: 0; display: flex; gap: 14px; }
.a-col { flex: 3; min-width: 0; }
.t-col { flex: 2; max-width: 480px; min-width: 0; }
</style>
