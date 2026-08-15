export function startOfDay(d: Date): Date {
  return new Date(d.getFullYear(), d.getMonth(), d.getDate())
}

/** يوم الأسبوع (0=السبت، 6=الجمعة) بالتوقيت المحلي */
export function dow(d: Date): number {
  return (d.getDay() + 1) % 7
}

/** أيام الأسبوع الثابتة (السبت→الجمعة): offset=0 الأسبوع الحالي، 1 السابق، إلخ */
export function daysOfWeek(offset: number): Date[] {
  const today = startOfDay(new Date())
  const start = new Date(today.getFullYear(), today.getMonth(), today.getDate() - dow(today) - offset * 7)
  return Array.from({ length: 7 }, (_, i) => new Date(start.getFullYear(), start.getMonth(), start.getDate() + i))
}

/** نطاق اليوم بالثواني [بداية اليوم، آخر ثانية فيه] */
export function dayRange(d: Date): [number, number] {
  const s = startOfDay(d)
  return [Math.floor(s.getTime() / 1000), Math.floor(s.getTime() / 1000) + 86400 - 1]
}

/** نطاق الشهر بالثواني [أول ثانية في الشهر، آخر ثانية فيه] */
export function monthRange(d: Date): [number, number] {
  const first = new Date(d.getFullYear(), d.getMonth(), 1)
  const last = new Date(d.getFullYear(), d.getMonth() + 1, 0)
  return [Math.floor(first.getTime() / 1000), Math.floor(last.getTime() / 1000) + 86400 - 1]
}

/** نطاق السنة بالثواني [أول ثانية في السنة، آخر ثانية فيها] */
export function yearRange(d: Date): [number, number] {
  const first = new Date(d.getFullYear(), 0, 1)
  const last = new Date(d.getFullYear(), 11, 31)
  return [Math.floor(first.getTime() / 1000), Math.floor(last.getTime() / 1000) + 86400 - 1]
}
