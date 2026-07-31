<script setup lang="ts">
import { computed } from 'vue'
import { startOfDay } from '../lib/dates'
import { categoryColor, categoryLabel, formatDuration, eventDuration, type LogEntry } from '../lib/dbus'

const props = defineProps<{ days: Date[]; logs: LogEntry[]; selected: Date }>()
const emit = defineEmits<{ select: [Date]; prev: []; next: [] }>()

const WEEKDAYS = ['سبت', 'أحد', 'إثن', 'ثلا', 'أرب', 'خمي', 'جمع']
const MONTHS = ['يناير', 'فبراير', 'مارس', 'أبريل', 'مايو', 'يونيو', 'يوليو', 'أغسطس', 'سبتمبر', 'أكتوبر', 'نوفمبر', 'ديسمبر']

const weekLabel = computed(() => {
  const [a, b] = [props.days[0], props.days[6]]
  return `${a.getDate()} ${MONTHS[a.getMonth()]} – ${b.getDate()} ${MONTHS[b.getMonth()]}`
})

const today = startOfDay(new Date())

function dayLogs(day: Date): LogEntry[] {
  return props.logs.filter(l => startOfDay(new Date(l.start_time * 1000)).getTime() === day.getTime())
}

function daySegs(day: Date): [string, number][] {
  const m = new Map<string, number>()
  for (const l of dayLogs(day)) {
    const cat = l.category || 'other'
    m.set(cat, (m.get(cat) ?? 0) + eventDuration(l))
  }
  return [...m.entries()].sort((a, b) => b[1] - a[1])
}

function dayTotal(day: Date): number {
  return dayLogs(day).reduce((s, l) => s + eventDuration(l), 0)
}

function isToday(d: Date) { return d.getTime() === today.getTime() }
function isSelected(d: Date) { return d.getTime() === props.selected.getTime() }
</script>

<template>
  <div class="w-card card">
    <div class="w-head">
      <button class="icon-btn" aria-label="الأسبوع السابق" @click="emit('prev')">‹</button>
      <span class="w-label">الأسبوع {{ weekLabel }}</span>
      <button class="icon-btn" aria-label="الأسبوع التالي" @click="emit('next')">›</button>
    </div>
    <div class="w-grid">
      <div v-for="(day, i) in days" :key="day.getTime()" class="w-day" :class="{ on: isSelected(day) }"
        :title="`${WEEKDAYS[i]} ${day.getDate()} ${MONTHS[day.getMonth()]} — ${formatDuration(dayTotal(day))}`"
        @click="emit('select', day)">
        <div class="w-bars">
          <template v-if="daySegs(day).length">
            <div v-for="[cat, d] in daySegs(day)" :key="'d' + day.getTime() + cat" class="w-seg"
              :style="{ flexGrow: Math.max(1, d), background: categoryColor(cat) }"></div>
          </template>
          <div v-else class="w-empty"></div>
        </div>
        <span class="w-dow">{{ WEEKDAYS[i] }}</span>
        <span class="w-num" :class="{ today: isToday(day) }">{{ day.getDate() }}</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.w-card { padding: 0.8rem 1rem; flex-shrink: 0; }
.w-head { display: flex; align-items: center; gap: 0.6rem; }
.w-label { flex: 1; text-align: center; font-weight: 700; font-size: 0.85rem; }
.w-grid { display: flex; gap: 8px; margin-top: 0.6rem; }
.w-day {
  flex: 1; display: flex; flex-direction: column; align-items: center; gap: 3px;
  padding: 6px 2px 8px; border-radius: 10px; border: 2px solid transparent; cursor: pointer;
}
.w-day:hover { background: var(--surface-soft); }
.w-day.on { border-color: var(--accent); background: var(--surface-soft); }
.w-bars { display: flex; flex-direction: column; gap: 2px; width: 100%; height: 56px; justify-content: flex-end; }
.w-seg { border-radius: 3px; min-height: 3px; }
.w-empty { flex: 1; background: var(--surface-soft); border-radius: 3px; min-height: 3px; }
.w-dow { font-size: 0.65rem; color: var(--ink-muted); font-weight: 700; }
.w-num { font-size: 0.7rem; color: var(--ink-muted); }
.w-num.today { color: var(--accent); font-weight: 900; }
</style>
