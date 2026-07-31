<script setup lang="ts">
import { computed } from 'vue'
import { startOfDay, dow } from '../lib/dates'
import { formatDuration, eventDuration, type LogEntry } from '../lib/dbus'

const props = defineProps<{ days: Date[]; logs: LogEntry[]; dayLogs: LogEntry[]; selected: Date; limits: [string, string, number][]; history: LogEntry[] }>()
const emit = defineEmits<{ select: [Date]; prev: []; next: [] }>()

const HISTORY_WEEKS = 8

const WEEKDAYS_FULL = ['السبت', 'الأحد', 'الاثنين', 'الثلاثاء', 'الأربعاء', 'الخميس', 'الجمعة']
const MONTHS = ['يناير', 'فبراير', 'مارس', 'أبريل', 'مايو', 'يونيو', 'يوليو', 'أغسطس', 'سبتمبر', 'أكتوبر', 'نوفمبر', 'ديسمبر']

const weekLabel = computed(() => {
  const [a, b] = [props.days[0], props.days[6]]
  return `${a.getDate()} ${MONTHS[a.getMonth()]} – ${b.getDate()} ${MONTHS[b.getMonth()]}`
})

function dayLogs(day: Date): LogEntry[] {
  return props.logs.filter(l => startOfDay(new Date(l.start_time * 1000)).getTime() === day.getTime())
}

function dayTotal(day: Date): number {
  return dayLogs(day).reduce((s, l) => s + eventDuration(l), 0)
}

/** مجموع تجاوزات حدود اليوم: لكل حد (تطبيق/فئة) المستخدم − الحد، تُجمع الموجبات وتُقيد بإجمالي اليوم */
function dayOverage(day: Date): number {
  const logs = dayLogs(day)
  let over = 0
  for (const [kind, target, minutes] of props.limits) {
    const used = logs
      .filter(l => kind === 'app' ? l.app_name === target : (l.category || 'other') === target)
      .reduce((s, l) => s + eventDuration(l), 0)
    if (used > minutes * 60) over += used - minutes * 60
  }
  return Math.min(over, dayTotal(day))
}

const dayTotals = computed(() => props.days.map(dayTotal))
const maxTotal = computed(() => Math.max(0, ...dayTotals.value))

function barPx(total: number): number {
  if (!maxTotal.value) return 3
  return Math.max(3, Math.round((total / maxTotal.value) * 56))
}
function overPx(over: number, total: number): number {
  if (!maxTotal.value) return 0
  return Math.min(Math.round((over / maxTotal.value) * 56), barPx(total))
}

const weekTotal = computed(() => dayTotals.value.reduce((s, t) => s + t, 0))
const selectedTotal = computed(() => props.dayLogs.reduce((s, l) => s + eventDuration(l), 0))

/** متوسط استخدام نفس يوم الأسبوع (المحدد) عبر الأسابيع الثمانية السابقة */
function avgDay(day: Date): number {
  const wd = dow(day)
  const sum = props.history
    .filter(l => dow(new Date(l.start_time * 1000)) === wd)
    .reduce((s, l) => s + eventDuration(l), 0)
  return sum / HISTORY_WEEKS
}
const avgDayNow = computed(() => avgDay(props.selected))
const avgWeek = computed(() => props.history.reduce((s, l) => s + eventDuration(l), 0) / HISTORY_WEEKS)

function isToday(d: Date) { return d.getTime() === startOfDay(new Date()).getTime() }
function isSelected(d: Date) { return d.getTime() === props.selected.getTime() }

function dayTitle(i: number, day: Date): string {
  const t = dayTotal(day)
  const over = dayOverage(day)
  return `${WEEKDAYS_FULL[i]} ${day.getDate()} ${MONTHS[day.getMonth()]} — ${formatDuration(t)}${over > 0 ? ` · +${formatDuration(over)} تجاوز` : ''}`
}
</script>

<template>
  <div class="w-card card">
    <div class="w-head">
      <span class="w-title">نظرة عامة</span>
      <span class="w-nav">
        <button class="icon-btn" aria-label="الأسبوع السابق" @click="emit('prev')">‹</button>
        <span class="w-label">الأسبوع {{ weekLabel }}</span>
        <button class="icon-btn" aria-label="الأسبوع التالي" @click="emit('next')">›</button>
      </span>
    </div>

    <div class="w-stats">
      <div class="w-stat">
        <div class="w-stat-num">{{ formatDuration(selectedTotal) }}</div>
        <div class="w-stat-label">إجمالي اليوم</div>
      </div>
      <div class="w-stat-div"></div>
      <div class="w-stat">
        <div class="w-stat-num muted">{{ formatDuration(weekTotal) }}</div>
        <div class="w-stat-label">إجمالي الأسبوع</div>
      </div>
      <div class="w-stat-div"></div>
      <div class="w-stat sub">
        <div class="w-stat-num">{{ formatDuration(avgDayNow) }}</div>
        <div class="w-stat-label">متوسط اليوم</div>
      </div>
      <div class="w-stat-div"></div>
      <div class="w-stat sub">
        <div class="w-stat-num">{{ formatDuration(avgWeek) }}</div>
        <div class="w-stat-label">متوسط الأسبوع</div>
      </div>
    </div>

    <div class="w-grid">
      <div v-for="(day, i) in days" :key="day.getTime()" class="w-col" :class="{ on: isSelected(day) }"
        :title="dayTitle(i, day)" @click="emit('select', day)">
        <div class="w-chart">
          <div class="w-bar" :style="{ height: barPx(dayTotal(day)) + 'px' }">
            <div v-if="dayOverage(day) > 0" class="w-over"
              :style="{ height: overPx(dayOverage(day), dayTotal(day)) + 'px' }"></div>
          </div>
        </div>
        <span class="w-dow">{{ WEEKDAYS_FULL[i] }}</span>
        <span class="w-num" :class="{ today: isToday(day) }">{{ day.getDate() }}</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.w-card { padding: 1rem 1.2rem 0.85rem; flex-shrink: 0; border-radius: var(--radius); box-shadow: var(--shadow-sm); }
.w-head { position: relative; display: flex; justify-content: center; }
.w-title { position: absolute; inset-inline-start: 0; font-size: 0.75rem; font-weight: 800; color: var(--ink-muted); align-self: center; }
.w-nav { display: flex; align-items: center; gap: 0.6rem; }
.w-label { font-weight: 700; font-size: 0.85rem; }
.w-stats { display: flex; align-items: center; gap: 1rem; margin-top: 0.6rem; padding: 0.55rem 0.75rem; background: var(--surface-soft); border-radius: 10px; }
.w-stat { flex: 1; display: flex; flex-direction: column; align-items: center; gap: 2px; }
.w-stat-num { font-size: 1.5rem; font-weight: 900; line-height: 1.1; color: var(--accent); font-variant-numeric: tabular-nums; }
.w-stat-num.muted { color: var(--ink); }
.w-stat.sub .w-stat-num { font-size: 1rem; font-weight: 700; color: var(--ink-muted); }
.w-stat-label { font-size: 0.72rem; color: var(--ink-muted); font-weight: 600; }
.w-stat-div { width: 1px; height: 2rem; background: var(--border); }
.w-grid { display: flex; gap: 10px; margin-top: 0.65rem; }
.w-col {
  flex: 1; display: flex; flex-direction: column; align-items: center; gap: 4px;
  padding: 8px 4px 6px; border-radius: 10px; border: 2px solid transparent; cursor: pointer;
  transition: background 150ms ease, transform 150ms ease;
}
.w-col:hover { background: var(--surface-soft); transform: translateY(-2px); }
.w-col.on { background: var(--surface-soft); }
.w-col.on .w-bar { box-shadow: 0 0 0 2px var(--accent); }
.w-chart { display: flex; align-items: flex-end; justify-content: center; width: 100%; height: 56px; }
.w-bar {
  width: 70%; max-width: 26px; border-radius: 6px 6px 3px 3px; overflow: hidden;
  display: flex; flex-direction: column; justify-content: flex-start;
  background: linear-gradient(180deg, color-mix(in srgb, var(--accent) 70%, transparent), var(--accent));
}
.w-over {
  width: 100%; border-radius: 3px 3px 0 0;
  background: linear-gradient(180deg, color-mix(in srgb, var(--danger) 85%, transparent), var(--danger));
}
.w-dow { font-size: 0.72rem; color: var(--ink-muted); font-weight: 600; }
.w-num { font-size: 0.72rem; color: var(--ink-muted); }
.w-num.today { color: var(--accent); font-weight: 900; }
</style>
