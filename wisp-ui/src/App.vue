<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import AnalysisTab from './components/AnalysisTab.vue'
import Timeline from './components/Timeline.vue'
import WeekNav from './components/WeekNav.vue'
import SettingsPage from './components/SettingsPage.vue'
import { getTimeline, getLimits, eventDuration, categoryLabel, getSetting, getCategories, setCategoryCache, type LogEntry } from './lib/dbus'
import { startOfDay, daysOfWeek, dayRange, monthRange, yearRange } from './lib/dates'
import { setLocale, t } from './lib/i18n'

setLocale((typeof navigator !== 'undefined' && navigator.language?.slice(0,2) === 'en') ? 'en' : 'ar')

const view = ref<'dashboard' | 'settings'>('dashboard')
const weekOffset = ref(0)
const selectedDay = ref(startOfDay(new Date()))
const dayLogs = ref<LogEntry[]>([])
const weekLogs = ref<LogEntry[]>([])
const yearLogs = ref<LogEntry[]>([])
const nowMonthLogs = ref<LogEntry[]>([])
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
    // الأسبوع الحالي (السبت→الجمعة): يبدأ اليوم السبت
    const [wFrom] = dayRange(daysOfWeek(0)[0])
    const [, wTo] = dayRange(daysOfWeek(0)[6])
    weekLogs.value = await getTimeline(wFrom, wTo)
    const [yFrom, yTo] = yearRange(selectedDay.value)
    yearLogs.value = await getTimeline(yFrom, yTo)
    const [nmFrom, nmTo] = monthRange(new Date())
    nowMonthLogs.value = await getTimeline(nmFrom, nmTo)
    currentWeekLogs.value = weekLogs.value
    // 8 أسابيع كاملة تسبق الأسبوع الحالي (لأعمدة «آخر 8 أسابيع» والمتوسطات)
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
  setCategoryCache(await getCategories().catch(() => []))
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
  return yearRange(sel)
})

function selectDay(d: Date) {
  selectedDay.value = startOfDay(d)
  refresh()
}

/** مصدر أحداث الصفحة حسب الفترة: يوم/أسبوع/شهر */
const timelineLogs = computed(() => {
  if (period.value === 'day') return dayLogs.value
  if (period.value === 'week') return weekLogs.value
  return yearLogs.value
})

const ribbonSegs = computed(() => {
  const m = new Map<string, number>()
  for (const l of timelineLogs.value) {
    if (l.event_type === 'system') continue
    const cat = l.category || 'other'
    m.set(cat, (m.get(cat) ?? 0) + eventDuration(l))
  }
  return [...m.entries()].sort((a, b) => b[1] - a[1])
})

const PERIODS = ['day', 'week', 'month'] as const

function shiftSelectedYear(n: number) {
  const s = selectedDay.value
  selectedDay.value = new Date(s.getFullYear() + n, s.getMonth(), s.getDate())
  refresh()
}
/** تنقّل ‹ › حسب الفترة: يوم→أسبوع، أسبوع→ثابت (آخر 8 أسابيع)، شهر→سنة */
function prevPeriod() {
  if (period.value === 'day') { weekOffset.value++; refresh() }
  else if (period.value === 'month') shiftSelectedYear(-1)
}
function nextPeriod() {
  if (period.value === 'day') { weekOffset.value = Math.max(0, weekOffset.value - 1); refresh() }
  else if (period.value === 'month') shiftSelectedYear(1)
}

function toggleSettings() { view.value = view.value === 'settings' ? 'dashboard' : 'settings' }

function onKey(e: KeyboardEvent) {
  if ((e.ctrlKey || e.metaKey) && e.key === ',') { e.preventDefault(); toggleSettings() }
}
onMounted(() => window.addEventListener('keydown', onKey))
onUnmounted(() => window.removeEventListener('keydown', onKey))
</script>

<template>
  <div id="shell" class="rtl">
    <header class="hdr">
      <div class="brand">
        <svg viewBox="0 0 24 24" width="22" height="22" aria-hidden="true"><defs><linearGradient id="ring-g" x1="0" y1="0" x2="1" y2="0"><stop offset="0" stop-color="#e13057"/><stop offset="0.4" stop-color="#e13057"/><stop offset="0.68" stop-color="#ff9db8"/><stop offset="1" stop-color="#ff9db8" stop-opacity="0"/></linearGradient><linearGradient id="wisp-g" x1="0" y1="1" x2="1" y2="0"><stop offset="0" stop-color="#e94560"/><stop offset="0.55" stop-color="#ff7ba3"/><stop offset="1" stop-color="#ffc7da"/></linearGradient></defs><circle cx="10.5" cy="12" r="6.5" fill="none" stroke="url(#ring-g)" stroke-width="2.4"/><g fill="none" stroke="url(#wisp-g)" stroke-linecap="round"><path d="M16.3 11.2 C18.2 9.5 19.6 7.7 22.2 5.5" stroke-width="1.7"/><path d="M14.9 8.7 C16.6 7.2 18.5 5.7 20.6 3.5" stroke-width="1.1"/><path d="M12.7 6.8 C14 5.6 15.6 4.5 17.2 3.4" stroke-width="0.7"/></g><path d="M10.5 12 L13.6 9.9" stroke="#e13057" stroke-width="1.5" stroke-linecap="round"/><circle cx="10.5" cy="12" r="1.3" fill="#e13057"/></svg>
        <span class="brand-name">{{ t('app.brand') }}</span>
      </div>

      <div class="period-switch" role="group" aria-label="الفلترة الزمنية">
        <button v-for="pId in PERIODS" :key="pId" class="pill mini" :class="{ on: period === pId }" @click="period = pId">
          {{ t(`analysis.period.${pId}`) }}
        </button>
      </div>

      <button class="icon-btn gear" :class="{ on: view === 'settings' }" :aria-label="t('app.tab.settings')" @click="toggleSettings">
        <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/></svg>
      </button>

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
          <WeekNav :days="weekDays" :logs="weekLogs" :yearLogs="yearLogs" :nowMonthLogs="nowMonthLogs" :dayLogs="dayLogs" :selected="selectedDay" :limits="limits" :history="historyLogs" :curWeekLogs="currentWeekLogs" :weekOffset="weekOffset" :period="period"
            @select="selectDay" @prev="prevPeriod" @next="nextPeriod" />

          <div class="cols">
            <AnalysisTab class="a-col" :logs="timelineLogs" :range="analysisRange" :loading="loading"
              v-model:groupBy="groupBy" @search="searchQuery = $event" />
            <Timeline class="t-col" :logs="timelineLogs" :loading="loading" v-model:query="searchQuery" :ribbon="ribbonSegs" />
          </div>
        </div>

        <SettingsPage v-else key="settings" :limits="limits" :todayLogs="dayLogs"
          @back="view = 'dashboard'" @changed="refresh" />
      </Transition>
    </main>
  </div>
</template>

<style scoped>
.hdr { display: grid; grid-template-columns: 1fr auto 1fr; align-items: center; column-gap: 0.9rem; padding: 0.5rem 0 0.55rem; }
.brand { font-size: 1.05rem; font-weight: 900; color: var(--accent); display: inline-flex; align-items: center; gap: 0.5rem; letter-spacing: -0.02em; justify-self: start; }
.brand svg { flex-shrink: 0; }
.period-switch { display: inline-flex; gap: 0.15rem; background: var(--surface-soft); border-radius: 999px; padding: 0.15rem; }
.period-switch .pill { background: transparent; border: none; }
.period-switch .pill.on { background: var(--accent); color: var(--accent-ink); }
.pill.mini { padding: 0.2rem 0.7rem; font-size: 0.78rem; }
.gear { justify-self: end; }
.gear.on { background: var(--accent); border-color: var(--accent); color: var(--accent-ink); }
.btn.small { padding: 0.25rem 0.7rem; font-size: 0.75rem; }
.banner {
  grid-column: 1 / -1; margin-top: 0.5rem; display: flex; align-items: center; gap: 0.8rem;
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
