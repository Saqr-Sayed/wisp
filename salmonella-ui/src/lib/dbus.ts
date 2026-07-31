import { invoke } from '@tauri-apps/api/core'

export interface LogEntry {
  id: number
  event_type: string
  app_name: string
  window_title: string
  start_time: number
  end_time: number | null
  duration: number | null
}

function formatTime(ts: number): string {
  const d = new Date(ts * 1000)
  return d.toLocaleTimeString('ar-SA', { hour: '2-digit', minute: '2-digit' })
}

function formatDuration(secs: number | null): string {
  if (!secs) return '—'
  const m = Math.floor(secs / 60)
  const s = secs % 60
  return `${m}د ${s}ث`
}

export async function getTimeline(from: number, to: number): Promise<LogEntry[]> {
  return invoke('get_timeline', { from, to })
}

export async function getStatus() {
  return invoke('get_status')
}

export async function search(query: string): Promise<LogEntry[]> {
  return invoke('search', { query })
}

export { formatTime, formatDuration }
