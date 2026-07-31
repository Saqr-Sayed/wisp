<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { formatTime, formatDuration, eventDuration, categoryColor, categoryLabel, search, type LogEntry } from '../lib/dbus'

const props = defineProps<{ logs: LogEntry[]; loading: boolean }>()
const total = computed(() => props.logs.reduce((s, l) => s + eventDuration(l), 0))
const selected = ref<LogEntry | null>(null)

const query = ref('')
const results = ref<LogEntry[]>([])

watch(query, async (q) => {
  if (!q.trim()) { results.value = []; return }
  results.value = await search(q)
})

function rowStyle(cat: string) { return { background: categoryColor(cat) } }
</script>

<template>
  <div class="t-card card">
    <div class="t-head">
      <b>الخط الزمني</b>
      <span class="t-count">{{ query ? results.length : logs.length }} حدثاً</span>
    </div>
    <input v-model="query" class="t-search" placeholder="ابحث في سجل النشاط..." />

    <div class="t-list">
      <template v-if="query">
        <div v-for="r in results" :key="'s' + r.id" class="entry" @click="selected = r" :class="{ on: selected?.id === r.id }">
          <span class="time">{{ formatTime(r.start_time) }}</span>
          <span class="badge" :style="rowStyle(r.category)">{{ categoryLabel(r.category) }}</span>
          <span class="app">{{ r.friendly_name || r.app_name }}</span>
          <span v-if="r.site" class="site">{{ r.site }}</span>
          <span class="title">{{ r.window_title }}</span>
          <span class="duration">{{ formatDuration(eventDuration(r)) }}</span>
        </div>
        <div v-if="results.length === 0" class="empty">لا نتائج لـ «{{ query }}»</div>
      </template>

      <template v-else>
        <div v-if="selected" class="detail">
          <b>{{ selected.friendly_name || selected.app_name }}</b>
          <span v-if="selected.site" class="site">{{ selected.site }}</span>
          <span>{{ selected.window_title }}</span>
          <span class="time">{{ formatTime(selected.start_time) }} · {{ formatDuration(eventDuration(selected)) }}</span>
        </div>

        <template v-if="loading && logs.length === 0">
          <div v-for="n in 5" :key="n" class="skel-row"><div class="skel" style="height:0.9rem;width:100%"></div></div>
        </template>
        <template v-else>
          <div v-for="log in logs" :key="'r' + log.id" class="entry" @click="selected = log" :class="{ on: selected?.id === log.id }">
            <span class="time">{{ formatTime(log.start_time) }}</span>
            <span class="badge" :style="rowStyle(log.category)">{{ categoryLabel(log.category) }}</span>
            <span class="app">{{ log.friendly_name || log.app_name }}</span>
            <span v-if="log.site" class="site">{{ log.site }}</span>
            <span class="title">{{ log.window_title }}</span>
            <span class="duration">{{ formatDuration(eventDuration(log)) }}</span>
          </div>
          <div v-if="logs.length === 0" class="empty">🌙 لا توجد أحداث في هذه الفترة</div>
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
.t-search {
  margin: 0.6rem 1rem 0.2rem;
  background: var(--surface-soft); border: 1px solid var(--border); border-radius: 999px;
  padding: 0.45rem 1rem; color: var(--ink); font-family: inherit; font-size: 0.85rem;
}
.t-search::placeholder { color: var(--ink-muted); }
.t-list { flex: 1; min-height: 0; overflow-y: auto; padding: 0 1rem; }
.detail {
  background: var(--surface-soft); border-radius: 10px; padding: 0.6rem 0.9rem;
  margin: 0.6rem 0; display: flex; flex-direction: column; gap: 0.15rem;
  font-size: 0.85rem;
}
.detail .site { color: var(--accent); font-size: 0.8rem; }
.entry {
  display: flex; gap: 0.5rem; padding: 0.55rem 0.4rem; border-bottom: 1px solid var(--border);
  align-items: center; cursor: pointer; border-radius: 8px;
  transition: background 120ms;
}
.entry:hover { background: var(--surface-soft); }
.entry.on { background: var(--surface-soft); }
.time { color: var(--ink-muted); min-width: 4rem; font-size: 0.8rem; }
.app { font-weight: 700; font-size: 0.85rem; }
.site { color: var(--accent); font-size: 0.8rem; }
.title { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--ink-muted); font-size: 0.85rem; }
.duration { color: var(--ink-muted); min-width: 4rem; text-align: left; font-size: 0.8rem; }
.skel-row { padding: 0.7rem 0.4rem; border-bottom: 1px solid var(--border); }
</style>
