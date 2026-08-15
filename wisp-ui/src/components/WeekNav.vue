<script setup lang="ts">
import { computed } from 'vue'
import { startOfDay, dow, daysOfWeek, dayRange, monthRange } from '../lib/dates'
import { formatDuration, eventDuration, type LogEntry } from '../lib/dbus'
import { weeksCount, sameDowSum, hourScale, weekRangeLabel } from '../lib/overview'
import { t } from '../lib/i18n'

const props = defineProps<{ days: Date[]; logs: LogEntry[]; yearLogs: LogEntry[]; nowMonthLogs: LogEntry[]; dayLogs: LogEntry[]; selected: Date; limits: [string, string, number][]; history: LogEntry[]; curWeekLogs: LogEntry[]; weekOffset: number; period: 'day' | 'week' | 'month' }>()
const emit = defineEmits<{ select: [Date]; prev: []; next: [] }>()

const CHART_H = 120

const isCurrentWeek = computed(() => props.weekOffset === 0)
const weekTitle = computed(() => weekRangeLabel(props.days[0], props.days[6], isCurrentWeek.value, t))

interface Col { key: string; day: Date; label: string; sub: string; total: number; today: boolean; on: boolean; title: string; over: number }

function rangeTotal(src: LogEntry[], from: number, to: number): number {
  return src.filter(l => l.event_type !== 'system' && l.start_time >= from && l.start_time <= to).reduce((s, l) => s + eventDuration(l), 0)
}

/** أعمدة المخطط حسب الفترة: يوم→أيام الأسبوع المعروض، أسبوع→آخر 8 أسابيع، شهر→شهور السنة */
const columns = computed<Col[]>(() => {
  if (props.period === 'week') {
    const src = [...props.history, ...props.curWeekLogs]
    const nowMs = startOfDay(new Date()).getTime()
    return Array.from({ length: 8 }, (_, i) => {
      const offset = 7 - i // الأقدم → الحالي
      const start = daysOfWeek(offset)[0]
      const end = daysOfWeek(offset)[6]
      const from = dayRange(start)[0]
      const to = dayRange(end)[1]
      const inRange = (d: Date) => d.getTime() >= start.getTime() && d.getTime() <= end.getTime()
      return {
        key: 'w' + offset, day: start,
        label: `${start.getDate()}/${start.getMonth() + 1}`,
        sub: `${end.getDate()}/${end.getMonth() + 1}`,
        total: rangeTotal(src, from, to),
        today: inRange(new Date(nowMs)), on: inRange(props.selected),
        title: weekRangeLabel(start, end, offset === 0, t), over: 0,
      }
    })
  }
  if (props.period === 'month') {
    const y = props.selected.getFullYear()
    const now = startOfDay(new Date())
    return Array.from({ length: 12 }, (_, i) => {
      const start = new Date(y, i, 1)
      const end = new Date(y, i + 1, 0)
      const from = dayRange(start)[0]
      const to = dayRange(end)[1]
      return {
        key: 'm' + i, day: start,
        label: t('months.' + i), sub: '',
        total: rangeTotal(props.yearLogs, from, to),
        today: now.getFullYear() === y && now.getMonth() === i,
        on: props.selected.getFullYear() === y && props.selected.getMonth() === i,
        title: `${t('months.' + i)} ${y}`, over: 0,
      }
    })
  }
  return props.days.map((day, i) => ({
    key: 'd' + day.getTime(), day,
    label: t('weekdays.full.' + dow(day)), sub: String(day.getDate()),
    total: dayTotal(day),
    today: isToday(day), on: isSelected(day),
    title: dayTitle(day), over: dayOverage(day),
  }))
})

function dayLogs(day: Date): LogEntry[] {
  return props.yearLogs.filter(l => startOfDay(new Date(l.start_time * 1000)).getTime() === day.getTime())
}

function dayTotal(day: Date): number {
  return dayLogs(day).filter(l => l.event_type !== 'system').reduce((s, l) => s + eventDuration(l), 0)
}

/** مجموع تجاوزات حدود اليوم: لكل حد (تطبيق/فئة) المستخدم − الحد، تُجمع الموجبات وتُقيد بإجمالي اليوم */
function dayOverage(day: Date): number {
  const logs = dayLogs(day)
  let over = 0
  for (const [kind, target, minutes] of props.limits) {
    const used = logs
      .filter(l => l.event_type !== 'system' && (kind === 'app' ? l.app_name === target : (l.category || 'other') === target))
      .reduce((s, l) => s + eventDuration(l), 0)
    if (used > minutes * 60) over += used - minutes * 60
  }
  return Math.min(over, dayTotal(day))
}

const weekTotal = computed(() => props.logs.filter(l => l.event_type !== 'system').reduce((s, l) => s + eventDuration(l), 0))
const selectedTotal = computed(() => props.dayLogs.filter(l => l.event_type !== 'system').reduce((s, l) => s + eventDuration(l), 0))
const curWeekTotal = computed(() => props.curWeekLogs.filter(l => l.event_type !== 'system').reduce((s, l) => s + eventDuration(l), 0))
const yearTotal = computed(() => props.yearLogs.filter(l => l.event_type !== 'system').reduce((s, l) => s + eventDuration(l), 0))
const selMonthTotal = computed(() => {
  const [from, to] = monthRange(props.selected)
  return rangeTotal(props.yearLogs, from, to)
})
const nowMonthTotal = computed(() => props.nowMonthLogs.filter(l => l.event_type !== 'system').reduce((s, l) => s + eventDuration(l), 0))

/** تسمية المخطط حسب الفترة: يوم→الأسبوع المعروض، أسبوع→آخر 8 أسابيع، شهر→السنة */
const rangeLabel = computed(() => {
  if (props.period === 'week') return t('overview.last8Weeks')
  if (props.period === 'month') return String(props.selected.getFullYear())
  return weekTitle.value
})

/** البطاقة الأولى (المميزة): اليوم→اليوم المحدد، أسبوع→هذا الأسبوع، شهر→هذا الشهر */
const card1 = computed(() => {
  if (props.period === 'week') {
    return { label: t('overview.thisWeek'), total: curWeekTotal.value, avg: avgWeek.value, avgLabel: t('overview.avgWeekLabel') }
  }
  if (props.period === 'month') {
    return { label: t('overview.thisMonth'), total: selMonthTotal.value, avg: avgMonth.value, avgLabel: t('overview.avgMonthLabel') }
  }
  return { label: t('overview.today'), total: selectedTotal.value, avg: avgDayNow.value, avgLabel: t('overview.avgDayLabel') }
})

/** البطاقة الثانية: اليوم→هذا الأسبوع، أسبوع→هذا الشهر، شهر→هذه السنة */
const card2 = computed(() => {
  if (props.period === 'week') {
    return { label: t('overview.thisMonth'), total: nowMonthTotal.value, avg: avgMonth.value, avgLabel: t('overview.avgMonthLabel') }
  }
  if (props.period === 'month') {
    return { label: t('overview.thisYear'), total: yearTotal.value, avg: avgMonth.value, avgLabel: t('overview.avgMonthLabel') }
  }
  return { label: weekTitle.value, total: weekTotal.value, avg: avgWeek.value, avgLabel: t('overview.avgWeekLabel') }
})

/** نافذة المتوسطات: تاريخ الثمانية أسابيع + الأسبوع الحالي (مثبتة على الحاضر) */
const avgWindow = computed(() => [...props.history, ...props.curWeekLogs].filter(l => l.event_type !== 'system'))
const avgDivisor = computed(() => weeksCount(avgWindow.value))
const avgDayNow = computed(() => sameDowSum(avgWindow.value, props.selected) / avgDivisor.value)
const avgWeek = computed(() => avgWindow.value.reduce((s, l) => s + eventDuration(l), 0) / avgDivisor.value)
const avgMonth = computed(() => yearTotal.value / 12)

/** خط المتوسط في المخطط يتبع الفترة */
const avgLine = computed(() => props.period === 'week' ? avgWeek.value : props.period === 'month' ? avgMonth.value : avgDayNow.value)

const maxHours = computed(() => hourScale(Math.max(1, ...columns.value.map(c => c.total)) / 3600, avgLine.value / 3600))
/** خطوط الساعة: يوم→كل ساعتين، أسبوع→كل 5 ساعات، شهر→كل 10 ساعات */
const hourStep = computed(() => props.period === 'month' ? 10 : props.period === 'week' ? 5 : 2)
const hourTicks = computed(() => {
  const m = maxHours.value
  const t: number[] = []
  for (let h = 0; h <= m; h += hourStep.value) t.push(h)
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
const avgLinePct = computed(() => Math.min((avgLine.value / (maxHours.value * 3600)) * 100, 99))

function isToday(d: Date) { return d.getTime() === startOfDay(new Date()).getTime() }
function isSelected(d: Date) { return d.getTime() === props.selected.getTime() }

function dayTitle(day: Date): string {
  const total = dayTotal(day)
  const over = dayOverage(day)
  return `${t('weekdays.full.' + dow(day))} ${day.getDate()} ${t('months.' + day.getMonth())} — ${formatDuration(total)}${over > 0 ? ` · +${formatDuration(over)} ${t('overview.over')}` : ''}`
}
</script>

<template>
  <div class="w-card card">
    <div class="w-head">
      <span class="w-title">{{ t('overview.title') }}</span>
      <span class="w-range">{{ rangeLabel }}</span>
      <span class="w-nav">
        <button class="icon-btn" :disabled="period === 'week'" :aria-label="t('overview.prevWeek')" @click="emit('prev')">‹</button>
        <button class="icon-btn" :disabled="period === 'week'" :aria-label="t('overview.nextWeek')" @click="emit('next')">›</button>
      </span>
    </div>

    <div class="w-body">
      <div class="w-cards">
        <div class="w-mcard">
          <div class="w-mc-label">{{ card1.label }}</div>
          <div class="w-mc-num">{{ formatDuration(card1.total) }}</div>
          <div class="w-mc-avg">{{ card1.avgLabel }} {{ formatDuration(card1.avg) }}</div>
        </div>
        <div class="w-mcard">
          <div class="w-mc-label">{{ card2.label }}</div>
          <div class="w-mc-num muted">{{ formatDuration(card2.total) }}</div>
          <div class="w-mc-avg">{{ card2.avgLabel }} {{ formatDuration(card2.avg) }}</div>
        </div>
      </div>

      <div class="w-chartwrap">
        <div class="w-grid" :class="{ tight: period === 'month', wide: period === 'week' }">
          <div class="w-lines">
            <div v-for="h in hourTicks" :key="h" class="w-line" :style="{ bottom: (h / maxHours * 100) + '%' }"></div>
            <div class="w-avgline" :style="{ bottom: avgLinePct + '%' }"></div>
          </div>
          <div v-for="c in columns" :key="c.key" class="w-col" :class="{ on: c.on }"
            :title="c.title" @click="emit('select', c.day)">
            <div class="w-chart">
              <div class="w-bar" :style="{ height: barPx(c.total) + 'px' }">
                <div v-if="c.over > 0" class="w-over"
                  :style="{ height: overPx(c.over, c.total) + 'px' }"></div>
              </div>
            </div>
            <span class="w-dow" :class="{ today: c.today && period !== 'day' }">{{ c.label }}</span>
            <span v-if="c.sub" class="w-num" :class="{ today: c.today }">{{ c.sub }}</span>
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
.w-head { display: flex; align-items: center; gap: 0.6rem; }
.w-title { font-size: 0.75rem; font-weight: 800; color: var(--ink-muted); }
.w-range { font-size: 0.75rem; font-weight: 700; color: var(--ink-muted); opacity: 0.8; }
.w-nav { display: flex; align-items: center; gap: 0.4rem; margin-inline-start: auto; }
.w-nav .icon-btn:disabled { opacity: 0.35; cursor: default; }
.w-body { display: flex; gap: 1.25rem; margin-top: 0.75rem; }
.w-cards { display: flex; flex-direction: column; gap: 0.6rem; width: 200px; flex-shrink: 0; }
.w-mcard { flex: 1; display: flex; flex-direction: column; justify-content: center; gap: 2px; padding: 0.65rem 0.75rem; background: var(--surface-soft); border-radius: 10px; }
.w-mc-label { font-size: 0.72rem; color: var(--ink-muted); font-weight: 700; }
.w-mc-num { font-size: 1.5rem; font-weight: 900; line-height: 1.15; color: var(--accent); font-variant-numeric: tabular-nums; }
.w-mc-num.muted { color: var(--ink); }
.w-mc-avg { font-size: 0.72rem; color: var(--ink-muted); font-weight: 600; }
.w-chartwrap { flex: 1; display: flex; gap: 0.4rem; min-width: 0; }
.w-grid { flex: 1; position: relative; display: flex; gap: 10px; }
.w-grid.tight { gap: 2px; }
.w-grid.tight .w-col { padding-inline: 1px; }
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
.w-grid.wide .w-bar { max-width: 44px; }
.w-grid.tight .w-bar { max-width: 38px; }
.w-over { width: 100%; border-radius: 3px 3px 0 0; background: linear-gradient(180deg, color-mix(in srgb, var(--danger) 85%, transparent), var(--danger)); }
.w-dow { font-size: 0.72rem; color: var(--ink-muted); font-weight: 600; }
.w-dow.today { color: var(--accent); font-weight: 900; }
.w-num { font-size: 0.72rem; color: var(--ink-muted); }
.w-num.today { color: var(--accent); font-weight: 900; }
.w-axis { position: relative; width: 26px; height: 120px; margin-top: 8px; flex-shrink: 0; }
.w-tick { position: absolute; right: 0; transform: translateY(50%); font-size: 0.65rem; color: var(--ink-muted); font-variant-numeric: tabular-nums; }
</style>
