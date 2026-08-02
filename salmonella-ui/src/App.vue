<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import AnalysisTab from './components/AnalysisTab.vue'
import Timeline from './components/Timeline.vue'
import WeekNav from './components/WeekNav.vue'
import SettingsPage from './components/SettingsPage.vue'
import { getTimeline, getLimits, eventDuration, categoryLabel, type LogEntry } from './lib/dbus'
import { startOfDay, daysOfWeek, dayRange } from './lib/dates'
import { setLocale, t } from './lib/i18n'
import { getSetting } from './lib/dbus'

setLocale((typeof navigator !== 'undefined' && navigator.language?.slice(0,2) === 'en') ? 'en' : 'ar')

const view = ref<'dashboard' | 'settings'>('dashboard')
const weekOffset = ref(0)
const selectedDay = ref(startOfDay(new Date()))
const dayLogs = ref<LogEntry[]>([])
const weekLogs = ref<LogEntry[]>([])
const historyLogs = ref<LogEntry[]>([])
const currentWeekLogs = ref<LogEntry[]>([])
const limits = ref<[string, string, number][]>([])
const nowSec = ref(Math.floor(Date.now() / 1000))
const loading = ref(true)
const error = ref(false)
const groupBy = ref<'app' | 'category' | 'site' | 'series'>('category')
const period = ref<'day' | 'week' | 'month'>('day')
const searchQuery = ref('')

async function refresh() {
  try {
    nowSec.value = Math.floor(Date.now() / 1000)
    const [dFrom, dTo] = dayRange(selectedDay.value)
    dayLogs.value = await getTimeline(dFrom, dTo)
    const [wFrom] = dayRange(weekDays.value[0])
    const [, wTo] = dayRange(weekDays.value[6])
    weekLogs.value = await getTimeline(wFrom, wTo)
    const [cwFrom] = dayRange(daysOfWeek(0)[0])
    const [, cwTo] = dayRange(daysOfWeek(0)[6])
    currentWeekLogs.value = await getTimeline(cwFrom, cwTo)
    const [hFrom] = dayRange(daysOfWeek(8)[0])
    const [, hTo] = dayRange(daysOfWeek(1)[6])
    historyLogs.value = await getTimeline(hFrom, hTo)
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
    if (l.event_type === 'system') continue
    const d = eventDuration(l, nowSec.value)
    used.set(`category:${l.category || 'other'}`, (used.get(`category:${l.category || 'other'}`) ?? 0) + d)
    used.set(`app:${l.app_name}`, (used.get(`app:${l.app_name}`) ?? 0) + d)
  }
  const over = new Map<string, string>()
  for (const [target, kind, minutes] of limits.value) {
    const u = (used.get(`${kind}:${target}`) ?? 0) / 60
    if (u > minutes) over.set(`${kind}:${target}`, t('app.banner.exceeded', { target: evalLabel(target, kind), minutes }))
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
  const stored = await getSetting('language').catch(() => 'auto')
  setLocale((stored as 'auto' | 'ar' | 'en') ?? 'auto')
  await refresh()
  timer = window.setInterval(async () => { await refresh(); notify() }, 5000)
})
onUnmounted(() => window.clearInterval(timer))

const weekDays = computed(() => daysOfWeek(weekOffset.value))

const analysisRange = computed<[number, number]>(() => {
  const sel = selectedDay.value
  if (period.value === 'day') return dayRange(sel)
  if (period.value === 'week') {
    const wd = weekDays.value
    return [dayRange(wd[0])[0], dayRange(wd[6])[1]]
  }
  const first = new Date(sel.getFullYear(), sel.getMonth(), 1)
  const last = new Date(sel.getFullYear(), sel.getMonth() + 1, 0)
  return [
    Math.floor(first.getTime() / 1000),
    Math.floor(last.getTime() / 1000) + 86399,
  ]
})

function selectDay(d: Date) {
  selectedDay.value = startOfDay(d)
  refresh()
}

const ribbonSegs = computed(() => {
  const m = new Map<string, number>()
  for (const l of dayLogs.value) {
    if (l.event_type === 'system') continue
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
        <div class="brand">
          <svg viewBox="0 0 24 24" width="34" height="34" aria-hidden="true"><defs><linearGradient id="rod-g" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#ff8fb2"/><stop offset="1" stop-color="#e13057"/></linearGradient></defs><g transform="rotate(-12 12 12)"><g fill="none" stroke="#ff8fb2" stroke-width="0.7" stroke-linecap="round"><path d="M16.6 10.2c1.3-.5 2.2.2 3.4-.2"/><path d="M16.6 12c1.4-.2 2.3.5 3.7.1"/><path d="M16.6 13.8c1.3.4 2.2-.2 3.4.5"/></g><rect x="1.6" y="6.9" width="21.2" height="10.2" rx="5.1" fill="#fff" opacity=".9"/><rect x="2.6" y="7.9" width="19.2" height="8.2" rx="4.1" fill="url(#rod-g)"/><rect x="4.2" y="8.8" width="14.6" height="2.7" rx="1.35" fill="#fff" opacity=".22"/><circle cx="12" cy="12" r="4.2" fill="#faf6ef"/><g stroke="#e94560" stroke-width="0.6" stroke-linecap="round"><line x1="12" y1="7.8" x2="12" y2="9"/><line x1="16.2" y1="12" x2="15" y2="12"/><line x1="12" y1="16.2" x2="12" y2="15"/><line x1="7.8" y1="12" x2="9" y2="12"/></g><g stroke="#e94560" stroke-width="0.7" stroke-linecap="round"><line x1="12" y1="12" x2="13.9" y2="10.9"/><line x1="12" y1="12" x2="10.7" y2="11.3"/></g><circle cx="12" cy="12" r="0.8" fill="#e94560"/></g></svg>
          <span class="brand-name">{{ t('app.brand') }}</span>
        </div>
        <button v-if="view === 'dashboard'" class="icon-btn gear" :aria-label="t('settings.title')" @click="view = 'settings'">
          <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/></svg>
        </button>
      </div>

      <div v-if="error" class="banner" role="alert">
        {{ t('app.error.fetch') }}
        <button class="btn primary small" @click="refresh">{{ t('app.error.retry') }}</button>
      </div>
      <div v-if="overSet.size" class="banner" role="alert">
        ⚠ {{ [...overSet.values()].join(' · ') }}
      </div>
    </header>

    <main>
      <Transition name="view" mode="out-in">
        <div v-if="view === 'dashboard'" key="dash" class="dash">
          <WeekNav :days="weekDays" :logs="weekLogs" :dayLogs="dayLogs" :selected="selectedDay" :limits="limits" :history="historyLogs" :curWeekLogs="currentWeekLogs" :weekOffset="weekOffset"
            @select="selectDay" @prev="weekOffset++; refresh()"
            @next="weekOffset = Math.max(0, weekOffset - 1); refresh()" />

          <div class="cols">
            <AnalysisTab class="a-col" :logs="dayLogs" :range="analysisRange" :loading="loading"
              :period="period" @update:period="period = $event" v-model:groupBy="groupBy" />
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
.hdr { position: relative; padding: 1rem 0 0.6rem; }
.hdr-row { display: flex; align-items: center; justify-content: center; }
.hdr .gear { position: absolute; inset-inline-end: 0; top: 50%; transform: translateY(-50%); }
.brand { font-size: 1.9rem; font-weight: 900; color: var(--accent); display: inline-flex; align-items: center; justify-content: center; gap: 0.6rem; }
.brand svg { flex-shrink: 0; }
.brand-name { letter-spacing: -0.02em; }
.btn.small { padding: 0.25rem 0.7rem; font-size: 0.75rem; }
.banner {
  margin-top: 0.5rem; display: flex; align-items: center; gap: 0.8rem;
  background: var(--danger-soft); border: 1px solid var(--danger); border-radius: var(--radius-sm);
  padding: 0.45rem 0.9rem; color: var(--danger); font-size: 0.85rem; font-weight: 600;
  animation: banner-in 200ms ease;
}
@keyframes banner-in { from { opacity: 0; transform: translateY(-6px); } to { opacity: 1; transform: translateY(0); } }

.dash { flex: 1; min-height: 0; display: flex; flex-direction: column; gap: 14px; padding-bottom: 1rem; }
.cols { flex: 1; min-height: 0; display: flex; gap: 14px; }
.a-col { flex: 3; min-width: 0; }
.t-col { flex: 2; max-width: 480px; min-width: 0; }
</style>
