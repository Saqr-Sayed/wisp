import { startOfDay, dow } from './dates.ts'

/** بداية الأسبوع المحلي (السبت) بالمللي ثانية */
export function weekStart(ts: number): number {
  const d = startOfDay(new Date(ts * 1000))
  d.setDate(d.getDate() - dow(d))
  return d.getTime()
}

/** عدد الأسابيع المتميزة في السجلات، سقف cap، حد أدنى 1 (متوسطات تدريجية) */
export function weeksCount(logs: { start_time: number }[], cap = 8): number {
  const weeks = new Set<number>()
  for (const l of logs) weeks.add(weekStart(l.start_time))
  return Math.max(1, Math.min(cap, weeks.size))
}

/** مجموع مدد السجلات الواقعة في نفس يوم الأسبوع المحلي لـ target */
export function sameDowSum(logs: { start_time: number; duration?: number | null }[], target: Date): number {
  const wd = dow(target)
  return logs
    .filter(l => dow(new Date(l.start_time * 1000)) === wd)
    .reduce((s, l) => s + (l.duration ?? 0), 0)
}

/** سقف محور الساعات = ceil(أقصى قيم الأعمدة والمتوسط)، حد أدنى 1 */
export function hourScale(maxDayHours: number, avgDayHours: number): number {
  return Math.max(1, Math.ceil(Math.max(maxDayHours, avgDayHours)))
}

const MONTHS = ['يناير', 'فبراير', 'مارس', 'أبريل', 'مايو', 'يونيو', 'يوليو', 'أغسطس', 'سبتمبر', 'أكتوبر', 'نوفمبر', 'ديسمبر']

/** تسمية بطاقة الأسبوع: «هذا الأسبوع» أو نطاق التاريخ القصير
 *  يستقبل t من i18n.ts لتسمية الشهر والوضع الحالي؛ الافتراضي يستخدم MONTHS المحلية */
export function weekRangeLabel(a: Date, b: Date, isCurrent: boolean, t?: (k: string) => string): string {
  const tFn = t ?? ((k: string) => k === 'overview.thisWeek' ? 'هذا الأسبوع' : MONTHS[+k.split('.')[1]])
  if (isCurrent) return tFn('overview.thisWeek')
  const ma = tFn('months.' + a.getMonth())
  const mb = tFn('months.' + b.getMonth())
  if (a.getMonth() === b.getMonth()) return `${a.getDate()} – ${b.getDate()} ${ma}`
  return `${a.getDate()} ${ma} – ${b.getDate()} ${mb}`
}
