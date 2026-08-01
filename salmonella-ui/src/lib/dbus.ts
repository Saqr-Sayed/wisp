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

/** لون الفئة من متغيرات CSS (يتبع السمة الحالية تلقائياً) */
export function categoryColor(cat: string): string {
  const v = getComputedStyle(document.documentElement).getPropertyValue(`--cat-${cat}`).trim()
  return v || '#8a7f6e'
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

export interface CustomCategory { id: number; kind: 'app' | 'site'; target: string; display_name: string }

export async function listCustomCategories(): Promise<CustomCategory[]> {
  const rows = await invoke('list_custom_categories') as [number, string, string, string][]
  return rows.map(([id, kind, target, display_name]) => ({ id, kind, target, display_name }))
}
export async function addCustomCategory(kind: 'app' | 'site', target: string, display_name: string) {
  return invoke('add_custom_category', { kind, target, display_name })
}
export async function removeCustomCategory(id: number) {
  return invoke('remove_custom_category', { id })
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
export async function listIgnored(): Promise<[string, string][]> { return invoke('list_ignored') }
export async function ignoreTarget(kind: 'app' | 'site', target: string) { return invoke('ignore_target', { kind, target }) }
export async function unignoreTarget(kind: 'app' | 'site', target: string) { return invoke('unignore_target', { kind, target }) }
