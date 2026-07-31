<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import AnalysisTab from './components/AnalysisTab.vue'
import Timeline from './components/Timeline.vue'
import LimitsTab from './components/LimitsTab.vue'
import SettingsTab from './components/SettingsTab.vue'
import StatusCard from './components/StatusCard.vue'
import SearchBar from './components/SearchBar.vue'
import { getTimeline, getLimits, periodRange, eventDuration, categoryLabel, type Period, type LogEntry } from './lib/dbus'

const tab = ref<'analysis' | 'timeline' | 'limits' | 'settings'>('analysis')
const period = ref<Period>('day')
const offset = ref(0)
const logs = ref<LogEntry[]>([])
const limits = ref<[string, string, number][]>([])
const nowSec = ref(Math.floor(Date.now() / 1000))

async function refresh() {
  nowSec.value = Math.floor(Date.now() / 1000)
  const [from, to] = periodRange(period.value, offset.value)
  logs.value = await getTimeline(from, to)
  limits.value = await getLimits()
  evaluateLimits()
}

const overSet = ref(new Map<string, string>()) // key "kind:target" -> رسالة
let lastNotified = new Map<string, number>() // key -> epoch of last notification

function evalLabel(target: string, kind: string): string {
  if (kind === 'category') return categoryLabel(target)
  const hit = logs.value.find(l => l.app_name === target)
  return hit?.friendly_name ?? target
}

function evaluateLimits() {
  const used = new Map<string, number>()
  for (const l of logs.value) {
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
      try {
        invoke('notify', { body: msg })
        lastNotified.set(key, nowMs)
      } catch { /* ignore */ }
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
</script>

<template>
  <div id="shell" class="rtl">
    <header>
      <h1>Salmonella</h1>
      <nav class="tabs">
        <button :class="{ active: tab === 'analysis' }" @click="tab = 'analysis'">تحليل</button>
        <button :class="{ active: tab === 'timeline' }" @click="tab = 'timeline'">الخط الزمني</button>
        <button :class="{ active: tab === 'limits' }" @click="tab = 'limits'">الحدود</button>
        <button :class="{ active: tab === 'settings' }" @click="tab = 'settings'">الإعدادات</button>
      </nav>
      <div class="period">
        <button @click="offset++; refresh()">→</button>
        <span>{{ todayLabel }}</span>
        <button @click="offset = Math.max(0, offset - 1); refresh()">←</button>
        <select :value="period" @change="period = ($event.target as HTMLSelectElement).value as Period; offset = 0; refresh()">
          <option value="day">اليوم</option>
          <option value="week">الأسبوع</option>
          <option value="month">الشهر</option>
        </select>
      </div>
      <div v-if="overSet.size" class="over-banner">⚠ {{ [...overSet.values()].join(' · ') }}</div>
    </header>
    <main>
      <StatusCard :logs="logs" />
      <SearchBar />
      <AnalysisTab v-if="tab === 'analysis'" :logs="logs" :period="period" :offset="offset" />
      <Timeline v-if="tab === 'timeline'" :logs="logs" />
      <LimitsTab v-if="tab === 'limits'" :limits="limits" :logs="logs" @changed="refresh" />
      <SettingsTab v-if="tab === 'settings'" @changed="refresh" />
    </main>
  </div>
</template>

<style>
@import './style.css';
</style>
