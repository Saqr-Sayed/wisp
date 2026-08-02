<script setup lang="ts">
import { ref, watch, onMounted } from 'vue'
import { formatTime, formatDuration, eventDuration, categoryColor, categoryLabel, search, type LogEntry } from '../lib/dbus'
import { t } from '../lib/i18n'

const props = defineProps<{ logs: LogEntry[]; loading: boolean; query: string; ribbon: [string, number][] }>()
const emit = defineEmits<{ 'update:query': [string] }>()
const expandedId = ref<number | null>(null)

const results = ref<LogEntry[]>([])

function toggle(log: LogEntry) {
  expandedId.value = expandedId.value === log.id ? null : log.id
}

/** تسمية حدث النظام: ترجمة event.<detail> أو التفصيلة نفسها */
function eventLabel(log: LogEntry): string {
  const k = `event.${log.detail}`
  return t(k) !== k ? t(k) : log.detail
}

watch(() => props.query, async (q) => {
  if (!q.trim()) { results.value = []; return }
  results.value = await search(q)
}, { immediate: true })

function rowStyle(cat: string) { return { background: categoryColor(cat) } }

const listEl = ref<HTMLElement | null>(null)
const cardEl = ref<HTMLElement | null>(null)

function onWheel(e: WheelEvent) {
  const el = listEl.value
  if (!el) return
  e.preventDefault()
  const dy = e.deltaMode === WheelEvent.DOM_DELTA_LINE ? e.deltaY * 32 : e.deltaY
  el.scrollTop += dy
}

onMounted(() => {
  cardEl.value?.addEventListener('wheel', onWheel, { passive: false })
})
</script>

<template>
  <div ref="cardEl" class="t-card card">
    <div class="t-head">
      <b>{{ t('timeline.title') }}</b>
      <span class="t-count">{{ t('timeline.eventCount', { n: query ? results.length : logs.length }) }}</span>
    </div>
    <div v-if="ribbon.length" class="t-ribbon" role="img" :aria-label="t('timeline.ribbonLabel')">
      <div v-for="[cat, d] in ribbon" :key="'rib' + cat" class="rib-seg"
        :style="{ flexGrow: Math.max(1, d), background: categoryColor(cat) }"
        :title="categoryLabel(cat)"></div>
    </div>
    <input :value="query" @input="emit('update:query', ($event.target as HTMLInputElement).value)" class="t-search" :placeholder="t('timeline.searchPlaceholder')" />

    <div ref="listEl" class="t-list">
      <template v-if="query">
        <div v-for="r in results" :key="'s' + r.id" class="entry" :class="{ on: expandedId === r.id, sys: r.event_type === 'system' }" @click="toggle(r)">
          <div class="row">
            <span class="time">{{ formatTime(r.start_time) }}</span>
            <template v-if="r.event_type === 'system'">
              <span class="badge sys">{{ eventLabel(r) }}</span>
              <span class="title">{{ r.window_title }}</span>
              <span v-if="r.duration" class="duration">{{ formatDuration(r.duration) }}</span>
            </template>
            <template v-else>
              <span class="badge" :style="rowStyle(r.category)">{{ categoryLabel(r.category) }}</span>
              <span class="app">{{ r.friendly_name || r.app_name }}</span>
              <span v-if="r.site" class="site">{{ r.site }}</span>
              <span class="title">{{ r.window_title }}</span>
              <span class="duration">{{ formatDuration(eventDuration(r)) }}</span>
            </template>
          </div>
          <div v-if="expandedId === r.id" class="detail" @click.stop>
            <template v-if="r.event_type === 'system'">
              <span class="d-line">{{ eventLabel(r) }} — {{ formatTime(r.start_time) }}{{ r.duration ? ' → ' + formatTime(r.end_time as number) : '' }}</span>
              <span v-if="r.duration" class="d-line">{{ t('timeline.duration') }}: {{ formatDuration(r.duration) }}</span>
              <span v-if="r.window_title" class="d-line">{{ r.window_title }}</span>
            </template>
            <template v-else>
              <span class="d-line">{{ formatTime(r.start_time) }} → {{ r.end_time != null ? formatTime(r.end_time) : '—' }}</span>
              <span class="d-line">{{ t('timeline.duration') }}: {{ formatDuration(eventDuration(r)) }}</span>
              <span class="d-line">{{ categoryLabel(r.category) }}</span>
              <span v-if="r.site" class="d-line">{{ r.site }}</span>
              <span v-if="r.series || r.episode" class="d-line">{{ [r.series, r.episode].filter(Boolean).join(' · ') }}</span>
              <span class="d-line">{{ r.window_title }}</span>
            </template>
          </div>
        </div>
        <div v-if="results.length === 0" class="empty">{{ t('timeline.empty.results', { q: query }) }}</div>
      </template>

      <template v-else>
        <template v-if="loading && logs.length === 0">
          <div v-for="n in 5" :key="n" class="skel-row"><div class="skel" style="height:0.9rem;width:100%"></div></div>
        </template>
        <template v-else>
          <div v-for="log in logs" :key="'r' + log.id" class="entry" :class="{ on: expandedId === log.id, sys: log.event_type === 'system' }" @click="toggle(log)">
            <div class="row">
              <span class="time">{{ formatTime(log.start_time) }}</span>
              <template v-if="log.event_type === 'system'">
                <span class="badge sys">{{ eventLabel(log) }}</span>
                <span class="title">{{ log.window_title }}</span>
                <span v-if="log.duration" class="duration">{{ formatDuration(log.duration) }}</span>
              </template>
              <template v-else>
                <span class="badge" :style="rowStyle(log.category)">{{ categoryLabel(log.category) }}</span>
                <span class="app">{{ log.friendly_name || log.app_name }}</span>
                <span v-if="log.site" class="site">{{ log.site }}</span>
                <span class="title">{{ log.window_title }}</span>
                <span class="duration">{{ formatDuration(eventDuration(log)) }}</span>
              </template>
            </div>
            <div v-if="expandedId === log.id" class="detail" @click.stop>
              <template v-if="log.event_type === 'system'">
                <span class="d-line">{{ eventLabel(log) }} — {{ formatTime(log.start_time) }}{{ log.duration ? ' → ' + formatTime(log.end_time as number) : '' }}</span>
                <span v-if="log.duration" class="d-line">{{ t('timeline.duration') }}: {{ formatDuration(log.duration) }}</span>
                <span v-if="log.window_title" class="d-line">{{ log.window_title }}</span>
              </template>
              <template v-else>
                <span class="d-line">{{ formatTime(log.start_time) }} → {{ log.end_time != null ? formatTime(log.end_time) : '—' }}</span>
                <span class="d-line">{{ t('timeline.duration') }}: {{ formatDuration(eventDuration(log)) }}</span>
                <span class="d-line">{{ categoryLabel(log.category) }}</span>
                <span v-if="log.site" class="d-line">{{ log.site }}</span>
                <span v-if="log.series || log.episode" class="d-line">{{ [log.series, log.episode].filter(Boolean).join(' · ') }}</span>
                <span class="d-line">{{ log.window_title }}</span>
              </template>
            </div>
          </div>
          <div v-if="logs.length === 0" class="empty">{{ t('timeline.empty.events') }}</div>
        </template>
      </template>
    </div>
  </div>
</template>

<style scoped>
.t-card { flex: 1; min-height: 0; display: flex; flex-direction: column; overflow: hidden; }
.t-head {
  display: flex; align-items: center; gap: 0.6rem;
  padding: 0.8rem 1rem 0.6rem; border-bottom: 1px solid var(--border);
}
.t-head b { font-size: 0.95rem; }
.t-count { margin-right: auto; color: var(--ink-muted); font-size: 0.75rem; }
.t-ribbon { display: flex; gap: 3px; padding: 0.55rem 1rem 0.15rem; overflow: hidden; }
.rib-seg { height: 10px; border-radius: 4px; min-width: 3px; }
.t-search {
  margin: 0.6rem 1rem 0.2rem;
  background: var(--surface-soft); border: 1px solid var(--border); border-radius: 999px;
  padding: 0.45rem 1rem; color: var(--ink); font-family: inherit; font-size: 0.85rem;
}
.t-search::placeholder { color: var(--ink-muted); }
.t-list { flex: 1; min-height: 0; overflow-y: auto; padding: 0 1rem; }
.detail {
  background: var(--surface-soft); border-radius: 10px; padding: 0.6rem 0.9rem;
  margin: 0.3rem 0 0.5rem; display: flex; flex-direction: column; gap: 0.2rem;
  font-size: 0.8rem;
}
.d-line { color: var(--ink-muted); overflow-wrap: anywhere; }
.entry {
  display: flex; flex-direction: column; padding: 0.55rem 0.4rem; border-bottom: 1px solid var(--border);
  cursor: pointer; border-radius: 8px;
  transition: background 120ms;
}
.row { display: flex; gap: 0.5rem; align-items: center; min-width: 0; }
.entry:hover { background: var(--surface-soft); }
.entry.on { background: var(--surface-soft); }
.entry.sys .title { color: var(--ink); font-style: italic; }
.badge.sys { background: var(--surface-soft); color: var(--ink-muted); }
.time { color: var(--ink-muted); min-width: 4rem; font-size: 0.8rem; }
.app { font-weight: 700; font-size: 0.85rem; }
.site { color: var(--accent); font-size: 0.8rem; }
.title { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--ink-muted); font-size: 0.85rem; }
.duration { color: var(--ink-muted); min-width: 4rem; text-align: left; font-size: 0.8rem; }
.skel-row { padding: 0.7rem 0.4rem; border-bottom: 1px solid var(--border); }
</style>
