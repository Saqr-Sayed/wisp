<script setup lang="ts">
import { computed } from 'vue'
import { startOfDay } from '../lib/dates'
import { formatDuration, eventDuration, type LogEntry } from '../lib/dbus'
import { weeksCount, sameDowSum, hourScale, weekRangeLabel } from '../lib/overview'

const props = defineProps<{ days: Date[]; logs: LogEntry[]; dayLogs: LogEntry[]; selected: Date; limits: [string, string, number][]; history: LogEntry[]; curWeekLogs: LogEntry[]; weekOffset: number }>()
const emit = defineEmits<{ select: [Date]; prev: []; next: [] }>()

const CHART_H = 120
const WEEKDAYS_FULL = ['السبت', 'الأحد', 'الاثنين', 'الثلاثاء', 'الأربعاء', 'الخميس', 'الجمعة']
const MONTHS = ['يناير', 'فبراير', 'مارس', 'أبريل', 'مايو', 'يونيو', 'يوليو', 'أغسطس', 'سبتمبر', 'أكتوبر', 'نوفمبر', 'ديسمبر']

const isCurrentWeek = computed(() => props.weekOffset === 0)
const weekTitle = computed(() => weekRangeLabel(props.days[0], props.days[6], isCurrentWeek.value))

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
const weekTotal = computed(() => dayTotals.value.reduce((s, t) => s + t, 0))
const selectedTotal = computed(() => props.dayLogs.reduce((s, l) => s + eventDuration(l), 0))

/** نافذة المتوسطات: تاريخ الثمانية أسابيع + الأسبوع الحالي (مثبتة على الحاضر) */
const avgWindow = computed(() => [...props.history, ...props.curWeekLogs])
const avgDivisor = computed(() => weeksCount(avgWindow.value))
const avgDayNow = computed(() => sameDowSum(avgWindow.value, props.selected) / avgDivisor.value)
const avgWeek = computed(() => avgWindow.value.reduce((s, l) => s + eventDuration(l), 0) / avgDivisor.value)

const maxHours = computed(() => hourScale(Math.max(...dayTotals.value) / 3600, avgDayNow.value / 3600))
const hourTicks = computed(() => {
  const m = maxHours.value
  const t: number[] = []
  for (let h = 0; h <= m; h += 2) t.push(h)
  if (t[t.length - 1] !== m) t.push(m)
  return t
})

function barPx(total: number): number {
  return Math.max(3, Math.round((total / (maxHours.value * 3600)) * CHART_H))
}
function overPx(over: number, total: number): number {
  if (!maxHours.value) return 0
  return Math.min(Math.round((over / (maxHours.value * 3600)) * CHART_H), barPx(total))
}
const avgLinePct = computed(() => Math.min((avgDayNow.value / (maxHours.value * 3600)) * 100, 99))

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
        <button class="icon-btn" aria-label="الأسبوع التالي" @click="emit('next')">›</button>
      </span>
    </div>

    <div class="w-body">
      <div class="w-cards">
        <div class="w-mcard">
          <div class="w-mc-label">اليوم</div>
          <div class="w-mc-num">{{ formatDuration(selectedTotal) }}</div>
          <div class="w-mc-avg">متوسط اليوم: {{ formatDuration(avgDayNow) }}</div>
        </div>
        <div class="w-mcard">
          <div class="w-mc-label">{{ weekTitle }}</div>
          <div class="w-mc-num muted">{{ formatDuration(weekTotal) }}</div>
          <div class="w-mc-avg">متوسط الأسبوع: {{ formatDuration(avgWeek) }}</div>
        </div>
      </div>

      <div class="w-chartwrap">
        <div class="w-grid">
          <div class="w-lines">
            <div v-for="h in hourTicks" :key="h" class="w-line" :style="{ bottom: (h / maxHours * 100) + '%' }"></div>
            <div class="w-avgline" :style="{ bottom: avgLinePct + '%' }">
              <span class="w-avgtag">متوسط اليوم</span>
            </div>
          </div>
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
        <div class="w-axis">
          <span v-for="h in hourTicks" :key="h" class="w-tick" :style="{ bottom: (h / maxHours * 100) + '%' }">{{ h }}</span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.w-card { padding: 1rem 1.2rem 0.85rem; flex-shrink: 0; border-radius: var(--radius); box-shadow: var(--shadow-sm); }
.w-head { display: flex; align-items: center; justify-content: space-between; }
.w-title { font-size: 0.75rem; font-weight: 800; color: var(--ink-muted); }
.w-nav { display: flex; align-items: center; gap: 0.4rem; }
.w-body { display: flex; gap: 1.25rem; margin-top: 0.75rem; }
.w-cards { display: flex; flex-direction: column; gap: 0.6rem; width: 200px; flex-shrink: 0; }
.w-mcard { flex: 1; display: flex; flex-direction: column; justify-content: center; gap: 2px; padding: 0.65rem 0.75rem; background: var(--surface-soft); border-radius: 10px; }
.w-mc-label { font-size: 0.72rem; color: var(--ink-muted); font-weight: 700; }
.w-mc-num { font-size: 1.5rem; font-weight: 900; line-height: 1.15; color: var(--accent); font-variant-numeric: tabular-nums; }
.w-mc-num.muted { color: var(--ink); }
.w-mc-avg { font-size: 0.72rem; color: var(--ink-muted); font-weight: 600; }
.w-chartwrap { flex: 1; display: flex; gap: 0.4rem; min-width: 0; }
.w-grid { flex: 1; position: relative; display: flex; gap: 10px; }
.w-lines { position: absolute; inset-inline: 4px; top: 8px; height: 120px; pointer-events: none; }
.w-line { position: absolute; right: 0; left: 0; border-top: 1px solid var(--border); opacity: 0.6; }
.w-avgline { position: absolute; right: 0; left: 0; border-top: 2px dashed var(--accent); }
.w-avgtag { position: absolute; inset-inline-end: 0; transform: translateY(-100%); font-size: 0.62rem; color: var(--accent); font-weight: 700; background: var(--surface); padding-inline: 4px; border-radius: 4px; }
.w-col { flex: 1; display: flex; flex-direction: column; align-items: center; gap: 4px; padding: 8px 4px 6px; border-radius: 10px; border: 2px solid transparent; cursor: pointer; transition: background 150ms ease, transform 150ms ease; }
.w-col:hover { background: var(--surface-soft); transform: translateY(-2px); }
.w-col.on { background: var(--surface-soft); }
.w-col.on .w-bar { box-shadow: 0 0 0 2px var(--accent); }
.w-chart { display: flex; align-items: flex-end; justify-content: center; width: 100%; height: 120px; }
.w-bar { width: 70%; max-width: 26px; border-radius: 6px 6px 3px 3px; overflow: hidden; display: flex; flex-direction: column; justify-content: flex-start; background: linear-gradient(180deg, color-mix(in srgb, var(--accent) 70%, transparent), var(--accent)); }
.w-over { width: 100%; border-radius: 3px 3px 0 0; background: linear-gradient(180deg, color-mix(in srgb, var(--danger) 85%, transparent), var(--danger)); }
.w-dow { font-size: 0.72rem; color: var(--ink-muted); font-weight: 600; }
.w-num { font-size: 0.72rem; color: var(--ink-muted); }
.w-num.today { color: var(--accent); font-weight: 900; }
.w-axis { position: relative; width: 26px; height: 120px; margin-top: 8px; flex-shrink: 0; }
.w-tick { position: absolute; right: 0; transform: translateY(50%); font-size: 0.65rem; color: var(--ink-muted); font-variant-numeric: tabular-nums; }
</style>
