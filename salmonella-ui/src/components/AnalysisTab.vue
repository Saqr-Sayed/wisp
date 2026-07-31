<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { getReport, getSeries, periodRange, formatDuration, categoryLabel, type Period, type LogEntry } from '../lib/dbus'

const props = defineProps<{ logs: LogEntry[]; period: string; offset: number }>()
const groupBy = ref<'app' | 'category' | 'site' | 'series'>('app')
const report = ref<[string, number][]>([])
const series = ref<[string, string, number][]>([])

watch(() => [props.period, props.offset, props.logs, groupBy.value], async () => {
  const [from, to] = periodRange(props.period as Period, props.offset)
  if (groupBy.value === 'series') {
    series.value = await getSeries(from, to)
  } else {
    report.value = await getReport(from, to, groupBy.value)
  }
}, { immediate: true })

const total = ref(0)
watch(report, r => { total.value = r.reduce((s, [, d]) => s + d, 0) })

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
  <div class="analysis">
    <div class="groupby">
      <button v-for="g in (['app', 'category', 'site', 'series'] as const)" :key="g"
        :class="{ active: groupBy === g }" @click="groupBy = g">
        {{ g === 'app' ? 'التطبيقات' : g === 'category' ? 'الفئات' : g === 'site' ? 'المواقع' : 'المسلسلات' }}
      </button>
    </div>

    <div v-if="groupBy === 'series'" class="series">
      <div v-for="[s, a] in seriesAgg" :key="s" class="srow">
        <b>{{ s }}</b> — {{ a.eps }} حلقة · <span>{{ formatDuration(a.secs) }}</span>
      </div>
      <div v-if="series.length === 0" class="empty">لا حلقات في هذه الفترة</div>
    </div>

    <div v-else>
      <div v-for="[key, d] in report" :key="key" class="bar-row">
        <span class="bar-label">{{ label(groupBy, key) }}</span>
        <div class="bar-wrap"><div class="bar" :style="{ width: (total ? d / total * 100 : 0) + '%' }"></div></div>
        <span class="bar-val">{{ formatDuration(d) }} · {{ total ? Math.round(d / total * 100) : 0 }}%</span>
      </div>
      <div v-if="report.length === 0" class="empty">لا بيانات في هذه الفترة</div>
    </div>
  </div>
</template>

<style scoped>
.groupby { display: flex; gap: 0.5rem; margin-bottom: 1rem; }
.groupby button { background: #111; border: 1px solid #333; border-radius: 6px; padding: 0.3rem 0.8rem; color: #aaa; cursor: pointer; }
.groupby button.active { border-color: #e94560; color: #eee; }
.bar-row { display: flex; gap: 0.5rem; align-items: center; padding: 0.3rem 0; }
.bar-label { width: 8rem; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.bar-wrap { flex: 1; background: #1a1a1a; border-radius: 4px; height: 1.1rem; overflow: hidden; }
.bar { height: 100%; background: #e94560; }
.bar-val { width: 9rem; color: #888; font-size: 0.85rem; text-align: left; }
.srow { padding: 0.4rem; border-bottom: 1px solid #222; }
.empty { color: #666; text-align: center; padding: 2rem; }
</style>
