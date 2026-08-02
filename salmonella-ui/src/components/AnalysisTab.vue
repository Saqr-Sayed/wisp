<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue'
import { getReport, getSeries, formatDuration, categoryLabel, categoryColor, type LogEntry } from '../lib/dbus'
import { listCustomCategories, type CustomCategory } from '../lib/dbus'
import { t } from '../lib/i18n'

const props = defineProps<{
  logs: LogEntry[]
  range: [number, number]
  loading: boolean
  groupBy: 'category' | 'app' | 'site' | 'series'
  period: 'day' | 'week' | 'month'
}>()
const emit = defineEmits<{
  'update:groupBy': ['category' | 'app' | 'site' | 'series']
  'update:period': ['day' | 'week' | 'month']
}>()

const TABS = ['category', 'app', 'site', 'series'] as const
const PERIODS = ['day', 'week', 'month'] as const

function tabLabel(id: (typeof TABS)[number]): string {
  // "series" هو تبويب "محتوى"
  return id === 'series' ? t('analysis.tab.content') : t(`analysis.tab.${id}`)
}
const report = ref<[string, number][]>([])
const series = ref<[string, string, number][]>([])

const cats = ref<CustomCategory[]>([])
onMounted(async () => { cats.value = await listCustomCategories() })

watch(() => [props.range, props.logs, props.groupBy] as const, async () => {
  const [from, to] = props.range
  if (props.groupBy === 'series') {
    series.value = await getSeries(from, to)
  } else {
    report.value = await getReport(from, to, props.groupBy)
  }
}, { immediate: true })

function pct(secs: number): number {
  const total = report.value.reduce((s, [, d]) => s + d, 0)
  return total ? Math.round((secs / total) * 100) : 0
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
    <div class="tabs-row">
      <button v-for="tabId in TABS" :key="tabId"
        class="pill" :class="{ on: groupBy === tabId }"
        @click="emit('update:groupBy', tabId)">
        {{ tabLabel(tabId) }}
      </button>
      <span class="spacer"></span>
      <div class="period-switch" role="group" aria-label="الفلترة الزمنية">
        <button v-for="pId in PERIODS" :key="pId"
          class="pill mini" :class="{ on: period === pId }"
          @click="emit('update:period', pId)">
          {{ t(`analysis.period.${pId}`) }}
        </button>
      </div>
    </div>
    <ul class="custom-cat-hint" v-if="cats.length">
      <li v-for="c in cats" :key="c.id" :title="`${c.kind}: ${c.target}`">· {{ c.display_name }}</li>
    </ul>

    <div v-if="loading && report.length === 0 && series.length === 0" class="bars">
      <div v-for="n in 3" :key="n" class="skel" style="height:1.1rem;width:100%"></div>
    </div>

    <div v-else-if="groupBy === 'series'" class="series">
      <div v-for="[s, a] in seriesAgg" :key="s" class="srow">
        <b>{{ s }}</b>
        <span class="s-eps">{{ t('analysis.episodesCount', { n: a.eps }) }}</span>
        <span class="s-dur">{{ formatDuration(a.secs) }}</span>
      </div>
      <div v-if="series.length === 0" class="empty">{{ t('analysis.empty.episodes') }}</div>
    </div>

    <div v-else class="bars">
      <div v-for="[key, d] in report" :key="key" class="bar-row">
        <span class="bar-label">{{ label(groupBy, key) }}</span>
        <div class="bar-wrap">
          <div class="bar" :style="{ width: pct(d) + '%', background: groupBy === 'category' ? categoryColor(key) : 'var(--accent)' }"></div>
        </div>
        <span class="bar-val">{{ formatDuration(d) }} · {{ pct(d) }}%</span>
      </div>
      <div v-if="report.length === 0" class="empty">{{ t('analysis.empty.data') }}</div>
    </div>

  </div>
</template>

<style scoped>
.tabs-row { position: sticky; top: 0; z-index: 2; display: flex; align-items: center; gap: 0.5rem; flex-wrap: wrap; background: var(--surface); margin: -1.1rem -1.2rem 0.6rem; padding: 0.6rem 1.2rem; box-shadow: 0 1px 0 var(--border); }
.pill.mini { padding: 0.2rem 0.6rem; font-size: 0.75rem; }
.period-switch { display: inline-flex; gap: 0.15rem; align-self: center; background: var(--surface-soft); border-radius: 999px; padding: 0.15rem; }
.period-switch .pill { background: transparent; border: none; }
.period-switch .pill.on { background: var(--accent); color: #fff; }
.spacer { flex: 1; }
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
.custom-cat-hint { list-style: none; padding: 0.4rem 0 0; margin: 0; display: flex; gap: 0.6rem; flex-wrap: wrap; font-size: 0.78rem; color: var(--ink-muted); }
.custom-cat-hint li { padding: 0.15rem 0.55rem; background: var(--surface-soft); border-radius: 6px; }
</style>
