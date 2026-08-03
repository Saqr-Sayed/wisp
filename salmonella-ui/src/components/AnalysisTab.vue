<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { getReport, getContent, formatDuration, categoryLabel, categoryColor, type LogEntry, renameSeries } from '../lib/dbus'
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
  search: [string]
}>()

const TABS = ['category', 'app', 'site', 'series'] as const
const PERIODS = ['day', 'week', 'month'] as const

function tabLabel(id: (typeof TABS)[number]): string {
  // "series" هو تبويب "محتوى"
  return id === 'series' ? t('analysis.tab.content') : t(`analysis.tab.${id}`)
}

const report = ref<[string, number][]>([])
const content = ref<[string, string, string, number][]>([])   // bucket, series, name, seconds
const collapsed = ref<Set<string>>(new Set())                 // أسماء السلاسل المطوية

watch(() => [props.range, props.logs, props.groupBy] as const, async () => {
  const [from, to] = props.range
  if (props.groupBy === 'series') {
    content.value = await getContent(from, to)
  } else {
    report.value = await getReport(from, to, props.groupBy)
  }
}, { immediate: true })

function pct(secs: number): number {
  const total = report.value.reduce((s, [, d]) => s + d, 0)
  return total ? Math.round((secs / total) * 100) : 0
}

function label(g: string, key: string): string {
  if (g === 'category') return categoryLabel(key)
  return key
}

type Bucket = 'reading' | 'watching' | 'listening'
const SECTIONS: Bucket[] = ['reading', 'watching', 'listening']
interface EpisodeNode { name: string; secs: number }
interface ItemNode { name: string; secs: number; episodes?: EpisodeNode[] }

const AR_DIGITS = ['٠','١','٢','٣','٤','٥','٦','٧','٨','٩','۰','۱','۲','۳','۴','۵','۶','۷','۸','۹']

/// تسمية الحلقة → (موسم، حلقة) للترتيب الرقمي:
/// "SpongeBob S01E03" ← (1,3)، "الدرس 26" ← (0,26)، "Show 3x05" ← (3,5)؛ بلا تطابق ← (0,0).
function parseEpisode(label: string): [number, number] {
  const t = [...label].map(c => {
    const i = AR_DIGITS.indexOf(c)
    return i >= 0 ? String(i % 10) : c
  }).join('')
  const sxe = t.match(/^(.+?)\s*S(\d{1,2})E(\d{1,3})$/i)
  if (sxe) return [Number(sxe[2]), Number(sxe[3])]
  const axb = t.match(/^(?:(.+?)\s*)?(\d{1,2})x(\d{1,3})$/)
  if (axb) return [Number(axb[2]), Number(axb[3])]
  const kw = t.match(/(الحلقة|الدرس|الجزء)\s*(\d+)$/)
  if (kw) return [0, Number(kw[2])]
  const epn = t.match(/ep(?:\.|isode)?\s*(\d{1,3})$/i)
  if (epn) return [0, Number(epn[1])]
  return [0, 0]
}

const sections = computed<Record<Bucket, ItemNode[]>>(() => {
  const acc: Record<Bucket, ItemNode[]> = { reading: [], watching: [], listening: [] }
  for (const [bucket, series, name, secs] of content.value) {
    const list = acc[bucket as Bucket]
    if (!list) continue
    if (series) {
      let node = list.find(n => n.name === series)
      if (!node) { node = { name: series, secs: 0, episodes: [] }; list.push(node) }
      node.secs += secs
      let ep = node.episodes!.find(e => e.name === name)
      if (ep) ep.secs += secs
      else node.episodes!.push({ name, secs })
    } else {
      let node = list.find(n => n.name === name)
      if (node) node.secs += secs
      else list.push({ name, secs })
    }
  }
  for (const b of SECTIONS) {
    acc[b].sort((a, z) => z.secs - a.secs)
    for (const n of acc[b]) {
      if (n.episodes) {
        // ترتيب الحلقات رقمي دائماً — لا يتأثر بمبدّل الفرز
        n.episodes.sort((a, z) => {
          const [as, ae] = parseEpisode(a.name)
          const [zs, ze] = parseEpisode(z.name)
          return as - zs || ae - ze
        })
      }
    }
  }
  return acc
})

const sectionTotals = computed<Record<Bucket, number>>(() => {
  const out = { reading: 0, watching: 0, listening: 0 }
  for (const b of SECTIONS) out[b] = sections.value[b].reduce((s, n) => s + n.secs, 0)
  return out
})
const sectionCounts = computed<Record<Bucket, number>>(() => {
  const out = { reading: 0, watching: 0, listening: 0 }
  for (const b of SECTIONS) out[b] = sections.value[b].length
  return out
})

function toggleCollapse(name: string) {
  const next = new Set(collapsed.value)
  if (next.has(name)) next.delete(name)
  else next.add(name)
  collapsed.value = next
}

const editingSeriesName = ref<string | null>(null)   // node name in rename mode
const editSeriesValue = ref('')

async function startRenameSeries(name: string) {
  editingSeriesName.value = name
  editSeriesValue.value = name
}
async function saveRenameSeries() {
  const old = editingSeriesName.value
  const v = editSeriesValue.value.trim()
  editingSeriesName.value = null
  if (!old || !v || v === old) return
  await renameSeries(old, v)
  await refreshContent()
}
function cancelRenameSeries() { editingSeriesName.value = null }

async function refreshContent() {
  const [from, to] = props.range
  content.value = await getContent(from, to)
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

    <div class="a-content">
      <div v-if="loading && report.length === 0 && content.length === 0" class="bars">
        <div v-for="n in 3" :key="n" class="skel" style="height:1.1rem;width:100%"></div>
      </div>

      <div v-else-if="groupBy === 'series'" class="content">
        <section v-for="b in SECTIONS" :key="b" class="c-sec">
          <div class="sec-head">
            <b>{{ t(`analysis.section.${b}`) }}</b>
            <span class="sec-total">{{ formatDuration(sectionTotals[b]) }} · {{ t('analysis.section.items', { n: sectionCounts[b] }) }}</span>
          </div>
          <template v-if="sections[b].length">
            <div v-for="it in sections[b]" :key="it.name" class="c-item">
              <template v-if="it.episodes">
                <div class="srow" :aria-expanded="!collapsed.has(it.name)">
                  <span class="dash">-</span>
                  <button class="chevron" :class="{ open: !collapsed.has(it.name) }"
                    aria-label="toggle" @click="toggleCollapse(it.name)">▸</button>
                  <template v-if="editingSeriesName === it.name">
                    <input v-model="editSeriesValue" class="edit-input" :placeholder="t('analysis.edit.seriesPlaceholder')"
                      @keyup.enter="saveRenameSeries" @keyup.esc="cancelRenameSeries" @blur="cancelRenameSeries" />
                  </template>
                  <template v-else>
                    <b class="clickable" @click="emit('search', it.name)">{{ it.name }}</b>
                    <button class="icon-btn" :aria-label="t('analysis.edit.rename')" @click="startRenameSeries(it.name)">✎</button>
                  </template>
                  <span class="s-eps">{{ t('analysis.episodesCount', { n: it.episodes.length }) }}</span>
                  <span class="s-dur">{{ formatDuration(it.secs) }}</span>
                </div>
                <div v-if="!collapsed.has(it.name)" class="tree-child">
                  <div v-for="ep in it.episodes" :key="ep.name" class="srow ep" @click="emit('search', ep.name)">
                    <span class="ep-name">{{ ep.name }}</span>
                    <span class="s-dur">{{ formatDuration(ep.secs) }}</span>
                  </div>
                </div>
              </template>
              <div v-else class="srow" @click="emit('search', it.name)">
                <span class="dash">-</span>
                <b class="clickable">{{ it.name }}</b>
                <span class="s-dur">{{ formatDuration(it.secs) }}</span>
              </div>
            </div>
          </template>
          <div v-else class="empty">{{ t('analysis.empty.data') }}</div>
        </section>
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

  </div>
</template>

<style scoped>
.tabs-row { display: flex; align-items: center; gap: 0.5rem; flex-wrap: wrap; }
.pill.mini { padding: 0.2rem 0.6rem; font-size: 0.75rem; }
.period-switch { display: inline-flex; gap: 0.15rem; align-self: center; background: var(--surface-soft); border-radius: 999px; padding: 0.15rem; }
.period-switch .pill { background: transparent; border: none; }
.period-switch .pill.on { background: var(--accent); color: #fff; }
.spacer { flex: 1; }
.analysis { flex: 1; min-height: 0; display: flex; flex-direction: column; gap: 0.9rem; padding: 1.1rem 1.2rem; overflow: hidden; }
.a-content { flex: 1; min-height: 0; overflow-y: auto; display: flex; flex-direction: column; gap: 0.6rem; padding-inline-end: 0.3rem; }
.bars { display: flex; flex-direction: column; gap: 0.6rem; }
.bar-row { display: flex; gap: 0.6rem; align-items: center; }
.bar-label { width: 8rem; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 0.85rem; font-weight: 600; }
.bar-wrap { flex: 1; background: var(--surface-soft); border-radius: 999px; height: 0.7rem; overflow: hidden; }
.bar { height: 100%; border-radius: 999px; transition: width 400ms ease, background 150ms; }
.bar-val { width: 9rem; color: var(--ink-muted); font-size: 0.8rem; text-align: left; }
.srow { display: flex; gap: 0.7rem; align-items: center; padding: 0.45rem 0.2rem; font-size: 0.9rem; }
.srow b { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.s-eps { color: var(--accent); font-weight: 700; font-size: 0.8rem; }
.s-dur { color: var(--ink-muted); font-size: 0.8rem; }
.content { display: flex; flex-direction: column; gap: 0.9rem; }
.c-sec { display: flex; flex-direction: column; gap: 0.25rem; }
.sec-head { display: flex; align-items: baseline; justify-content: space-between; gap: 0.5rem; padding: 0.5rem 0 0.2rem; }
.sec-head b { font-size: 0.95rem; }
.sec-total { color: var(--ink-muted); font-size: 0.75rem; }
.dash { color: var(--ink-muted); font-weight: 700; width: 0.9rem; text-align: center; flex-shrink: 0; }
.srow .clickable { cursor: pointer; }
.srow .icon-btn { background: none; border: none; color: var(--ink-muted); cursor: pointer; padding: 0.15rem 0.3rem; font-size: 0.8rem; }
.srow .icon-btn:hover { color: var(--accent); }
.edit-input { flex: 1; min-width: 0; border: 1px solid var(--accent); border-radius: 6px; background: var(--surface); color: var(--ink); padding: 0.2rem 0.5rem; font-size: 0.85rem; }
.chevron { background: none; border: none; color: var(--ink-muted); cursor: pointer; padding: 0.2rem; font-size: 0.8rem; transition: transform 150ms; }
.chevron.open { transform: rotate(90deg); }
.tree-child { padding-inline-start: 1.8rem; display: flex; flex-direction: column; }
.srow.ep { position: relative; cursor: pointer; }
/* خط التوصيل العمودي — يمتد بطول الصف، ويتوقف عند منحنى آخر طفل */
.srow.ep::before {
  content: '';
  position: absolute;
  inset-inline-start: -0.6rem;
  top: 0; bottom: 0;
  width: 2px;
  background: var(--border);
  border-radius: 1px;
}
.srow.ep:last-child::before { bottom: 50%; }
/* الذراع الأفقي + الزاوية المستديرة عند منتصف الصف */
.srow.ep::after {
  content: '';
  position: absolute;
  inset-inline-start: -0.6rem;
  top: 50%;
  width: 0.6rem;
  height: 0.5rem;
  border-inline-start: 2px solid var(--border);
  border-bottom: 2px solid var(--border);
  border-end-start-radius: 8px;
}
.ep-name { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--ink-muted); font-size: 0.85rem; }
</style>
