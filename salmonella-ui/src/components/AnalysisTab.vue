<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { getReport, getSeries, periodRange, formatDuration, categoryLabel, categoryColor, type Period, type LogEntry } from '../lib/dbus'

const props = defineProps<{ logs: LogEntry[]; period: string; offset: number; loading: boolean; groupBy: 'app' | 'category' | 'site' | 'series' }>()
const emit = defineEmits<{ 'update:groupBy': ['app' | 'category' | 'site' | 'series'] }>()
const report = ref<[string, number][]>([])
const series = ref<[string, string, number][]>([])
const reportCat = ref<[string, number][]>([])
const reportApp = ref<[string, number][]>([])

watch(() => [props.period, props.offset, props.logs, props.groupBy] as const, async () => {
  const [from, to] = periodRange(props.period as Period, props.offset)
  if (props.groupBy === 'series') {
    series.value = await getSeries(from, to)
  } else {
    report.value = await getReport(from, to, props.groupBy)
  }
  reportCat.value = await getReport(from, to, 'category')
  reportApp.value = await getReport(from, to, 'app')
}, { immediate: true })

const sortedCat = computed(() => [...reportCat.value].sort((a, b) => b[1] - a[1]))
const sortedApp = computed(() => [...reportApp.value].sort((a, b) => b[1] - a[1]))

const totalSecs = computed(() => sortedCat.value.reduce((s, [, d]) => s + d, 0))
const totalLabel = computed(() => (props.period === 'day' ? 'إجمالي اليوم' : 'إجمالي الفترة'))
const topCat = computed(() => sortedCat.value[0])
const topApp = computed(() => sortedApp.value[0])

function topAppColor(): string {
  if (!topApp.value) return 'var(--accent)'
  const hit = props.logs.find(l => l.app_name === topApp.value![0])
  return categoryColor(hit?.category ?? '')
}

function pct(secs: number): number {
  return totalSecs.value ? Math.round((secs / totalSecs.value) * 100) : 0
}

const seriesAgg = computed(() => {
  const m = new Map<string, { eps: number; secs: number }>()
  for (const [s, , secs] of series.value) {
    const e = m.get(s) ?? { eps: 0, secs: 0 }
    e.eps++
    e.secs += secs
    m.set(s, e)
  }
  return [...m.entries()].sort((a, b) => b[1].secs - a[1].secs)
})

function label(g: string, key: string): string {
  if (g === 'category') return categoryLabel(key)
  return key
}
</script>

<template>
  <div class="analysis card">
    <div class="pill-group">
      <button v-for="g in (['app', 'category', 'site', 'series'] as const)" :key="g"
        class="pill" :class="{ on: groupBy === g }" @click="emit('update:groupBy', g)">
        {{ g === 'app' ? 'التطبيقات' : g === 'category' ? 'الفئات' : g === 'site' ? 'المواقع' : 'المسلسلات' }}
      </button>
    </div>

    <div v-if="loading && report.length === 0 && series.length === 0" class="bars">
      <div v-for="n in 3" :key="n" class="skel" style="height:1.1rem;width:100%"></div>
    </div>

    <div v-else-if="groupBy === 'series'" class="series">
      <div v-for="[s, a] in seriesAgg" :key="s" class="srow">
        <b>{{ s }}</b>
        <span class="s-eps">{{ a.eps }} حلقة</span>
        <span class="s-dur">{{ formatDuration(a.secs) }}</span>
      </div>
      <div v-if="series.length === 0" class="empty">📊 لا حلقات في هذه الفترة</div>
    </div>

    <div v-else class="bars">
      <div v-for="[key, d] in report" :key="key" class="bar-row">
        <span class="bar-label">{{ label(groupBy, key) }}</span>
        <div class="bar-wrap">
          <div class="bar" :style="{ width: pct(d) + '%', background: groupBy === 'category' ? categoryColor(key) : 'var(--accent)' }"></div>
        </div>
        <span class="bar-val">{{ formatDuration(d) }} · {{ pct(d) }}%</span>
      </div>
      <div v-if="report.length === 0" class="empty">📊 لا بيانات في هذه الفترة</div>
    </div>

    <div class="heroes">
      <div class="hero">
        <div class="hero-num" :key="totalSecs" :class="{ bump: totalSecs > 0 }">{{ formatDuration(totalSecs) }}</div>
        <div class="hero-label">{{ totalLabel }}</div>
      </div>
      <div class="hero">
        <div class="hero-num" :key="topCat ? topCat[1] : 0" :class="{ bump: topCat }" :style="{ color: topCat ? categoryColor(topCat[0]) : 'var(--ink-muted)' }">{{ topCat ? formatDuration(topCat[1]) : '—' }}</div>
        <div class="hero-label">{{ topCat ? categoryLabel(topCat[0]) : 'أعلى فئة' }}</div>
      </div>
      <div class="hero">
        <div class="hero-num" :key="topApp ? topApp[1] : 0" :class="{ bump: topApp }" :style="{ color: topApp ? topAppColor() : 'var(--ink-muted)' }">{{ topApp ? formatDuration(topApp[1]) : '—' }}</div>
        <div class="hero-label">{{ topApp ? topApp[0] : 'أعلى تطبيق' }}</div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.analysis { flex: 1; min-height: 0; display: flex; flex-direction: column; gap: 0.9rem; padding: 1.1rem 1.2rem; overflow-y: auto; }
.bars { display: flex; flex-direction: column; gap: 0.6rem; }
.bar-row { display: flex; gap: 0.6rem; align-items: center; }
.bar-label { width: 8rem; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 0.85rem; font-weight: 600; }
.bar-wrap { flex: 1; background: var(--surface-soft); border-radius: 999px; height: 0.7rem; overflow: hidden; }
.bar { height: 100%; border-radius: 999px; transition: width 400ms ease, background 150ms; }
.bar-val { width: 9rem; color: var(--ink-muted); font-size: 0.8rem; text-align: left; }
.srow { display: flex; gap: 0.7rem; align-items: center; padding: 0.45rem 0.2rem; border-bottom: 1px solid var(--border); font-size: 0.9rem; }
.srow b { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.s-eps { color: var(--accent); font-weight: 700; font-size: 0.8rem; }
.s-dur { color: var(--ink-muted); font-size: 0.8rem; }
.heroes { display: flex; gap: 10px; margin-top: auto; padding-top: 0.6rem; }
.hero { flex: 1; background: var(--surface-soft); border-radius: 10px; padding: 0.7rem 0.9rem; }
.hero-num { font-size: 1.4rem; font-weight: 900; line-height: 1.15; transition: transform 200ms ease; }
.hero-num.bump { animation: hero-bump 200ms ease; }
@keyframes hero-bump { 0% { transform: scale(1); } 40% { transform: scale(1.04); } 100% { transform: scale(1); } }
.hero-label { color: var(--ink-muted); font-size: 0.75rem; margin-top: 0.15rem; }
</style>
