import { invoke } from '@tauri-apps/api/core'
import { locale as i18nLocale, t } from './i18n'

export interface LogEntry {
  id: number
  event_type: string
  app_name: string
  window_title: string
  start_time: number
  end_time: number | null
  duration: number | null
  friendly_name: string
  site: string
  category: string
  series: string
  episode: string
  detail: string
}

export type Period = 'day' | 'week' | 'month'

export function formatTime(ts: number): string {
  const d = new Date(ts * 1000)
  const loc = i18nLocale.value === 'ar' ? 'ar-SA' : 'en-US'
  return d.toLocaleTimeString(loc, { hour: '2-digit', minute: '2-digit', numberingSystem: 'latn' })
}

export function formatDuration(secs: number | null): string {
  if (!secs) return '—'
  const m = Math.floor(secs / 60)
  const s = secs % 60
  if (m >= 60) {
    const h = Math.floor(m / 60)
    const rem = m % 60
    return i18nLocale.value === 'ar' ? `${h}س ${rem}د` : `${h}h ${rem}m`
  }
  return i18nLocale.value === 'ar' ? `${m}د ${s}ث` : `${m}m ${s}s`
}

export function periodRange(period: Period, offset: number): [number, number] {
  const now = new Date()
  if (period === 'day') {
    const d = new Date(now.getFullYear(), now.getMonth(), now.getDate() - offset)
    const start = new Date(d.getFullYear(), d.getMonth(), d.getDate(), 0, 0, 0)
    const end = new Date(d.getFullYear(), d.getMonth(), d.getDate(), 23, 59, 59)
    return [Math.floor(start.getTime() / 1000), Math.floor(end.getTime() / 1000)]
  }
  if (period === 'week') {
    const d = new Date(now.getFullYear(), now.getMonth(), now.getDate() - offset * 7)
    const start = new Date(d.getFullYear(), d.getMonth(), d.getDate() - 6, 0, 0, 0)
    const end = new Date(d.getFullYear(), d.getMonth(), d.getDate(), 23, 59, 59)
    return [Math.floor(start.getTime() / 1000), Math.floor(end.getTime() / 1000)]
  }
  const m = new Date(now.getFullYear(), now.getMonth() - offset, 1)
  const start = new Date(m.getFullYear(), m.getMonth(), 1, 0, 0, 0)
  const end = new Date(m.getFullYear(), m.getMonth() + 1, 0, 23, 59, 59)
  return [Math.floor(start.getTime() / 1000), Math.floor(end.getTime() / 1000)]
}

export interface CategoryInfo {
  id: number; name: string; color: string; is_builtin: number; is_deletable: number; sort: number
}
export type CategoryMember = { kind: 'app' | 'site'; target: string }

let catColorCache = new Map<string, string>()

/** تُستدعى بعد فتح التطبيق أو بعد تعديل الفئات لتحديث لون الرمز. */
export function setCategoryCache(cats: CategoryInfo[]) {
  catColorCache = new Map(cats.map(c => [c.name, c.color]))
}

export function categoryColor(cat: string): string {
  return catColorCache.get(cat) ?? '#8a7f6e'
}

export async function getCategories(): Promise<CategoryInfo[]> {
  const rows = await invoke('get_categories') as [number, string, string, number, number, number][]
  return rows.map(([id, name, color, is_builtin, is_deletable, sort]) =>
    ({ id, name, color, is_builtin, is_deletable, sort }))
}
export async function getCategoryMembers(id: number): Promise<[string, string][]> {
  return invoke('get_category_members', { id })
}
export async function addCategory(name: string, color: string): Promise<number> {
  return invoke('add_category', { name, color })
}
export async function renameCategory(id: number, newName: string) {
  return invoke('rename_category', { id, newName })
}
export async function setCategoryColor(id: number, color: string) {
  return invoke('set_category_color', { id, color })
}
export async function addCategoryMember(id: number, kind: string, target: string) {
  return invoke('add_category_member', { id, kind, target })
}
export async function deleteCategoryMember(kind: string, target: string) {
  return invoke('delete_category_member', { kind, target })
}
export async function deleteCategory(id: number): Promise<boolean> {
  return invoke('delete_category', { id })
}

export const CATEGORY_LABELS: Record<string, string> = {
  media: 'وسائط', reading: 'قراءة', games: 'ألعاب', entertainment: 'ترفيه',
  productivity: 'إنتاجية', browsing: 'تصفح', other: 'أخرى',
}

export function categoryLabel(c: string): string {
  const k = `category.${c}`
  return t(k) !== k ? t(k) : (CATEGORY_LABELS[c] ?? c)
}

/** مدة الحدث محتسبةً الحدث الجاري (duration ?? now - start_time) */
export function eventDuration(e: LogEntry, nowSec = Math.floor(Date.now() / 1000)): number {
  return e.duration ?? Math.max(0, nowSec - e.start_time)
}

export async function getTimeline(from: number, to: number): Promise<LogEntry[]> {
  return invoke('get_timeline', { from, to })
}
export async function search(query: string): Promise<LogEntry[]> { return invoke('search', { query }) }
export async function getReport(from: number, to: number, groupBy: string): Promise<[string, number][]> {
  return invoke('get_report', { from, to, groupBy })
}
export async function getSeries(from: number, to: number): Promise<[string, string, number][]> {
  return invoke('get_series', { from, to })
}
export async function getContent(from: number, to: number): Promise<[string, string, string, number][]> {
  return invoke('get_content', { from, to })
}
export async function getLimits(): Promise<[string, string, number][]> { return invoke('get_limits') }
export async function setLimit(target: string, kind: string, minutes: number) { return invoke('set_limit', { target, kind, minutes }) }
export async function removeLimit(target: string) { return invoke('remove_limit', { target }) }
export async function getNameOverrides(): Promise<[string, string][]> { return invoke('get_name_overrides') }
export async function setNameOverride(appId: string, friendly: string) { return invoke('set_name_override', { appId, friendly }) }
export async function removeNameOverride(appId: string) { return invoke('remove_name_override', { appId }) }
export async function getSetting(key: string): Promise<string> {
  return invoke('get_setting', { key })
}
export async function setSetting(key: string, value: string): Promise<void> {
  return invoke('set_setting', { key, value })
}

export interface KnownApp { id: string; display: string; overridden: boolean }
export interface KnownSite { site: string; display: string; overridden: boolean }

export async function getKnownApps(): Promise<KnownApp[]> {
  const rows = await invoke('get_known_apps') as [string, string][]
  const overrides = new Map(await getNameOverrides())
  return rows.map(([id, display]) => ({ id, display, overridden: overrides.has(id) }))
}
export async function getKnownSites(): Promise<KnownSite[]> {
  const rows = await invoke('get_known_sites') as [string, string][]
  const overrides = new Map(await getSiteOverrides())
  return rows.map(([site, display]) => ({ site, display, overridden: overrides.has(site) }))
}
export async function getSiteOverrides(): Promise<[string, string][]> { return invoke('get_site_overrides') }
export async function setSiteOverride(site: string, friendly: string) { return invoke('set_site_override', { site, friendly }) }
export async function removeSiteOverride(site: string) { return invoke('remove_site_override', { site }) }
export async function getSeriesOverrides(): Promise<[string, string][]> { return invoke('get_series_overrides') }
export async function setSeriesOverride(pattern: string, name: string) { return invoke('set_series_override', { pattern, name }) }
export async function removeSeriesOverride(pattern: string) { return invoke('remove_series_override', { pattern }) }
export async function renameSeries(old: string, newName: string) { return invoke('rename_series', { old, new: newName }) }
export async function clearSeries(title: string) { return invoke('clear_series', { title }) }
export async function listIgnored(): Promise<[string, string][]> { return invoke('list_ignored') }
export async function ignoreTarget(kind: 'app' | 'site', target: string) { return invoke('ignore_target', { kind, target }) }
export async function unignoreTarget(kind: 'app' | 'site', target: string) { return invoke('unignore_target', { kind, target }) }
export async function archiveTarget(kind: 'app' | 'site', target: string) { return invoke('archive_target', { kind, target }) }
export async function unarchiveTarget(kind: 'app' | 'site', target: string) { return invoke('unarchive_target', { kind, target }) }
export async function getArchived(): Promise<[string, string][]> { return invoke('list_archived') }
