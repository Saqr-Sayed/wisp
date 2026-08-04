export function startOfDay(d: Date): Date {
  return new Date(d.getFullYear(), d.getMonth(), d.getDate())
}

/** يوم الأسبوع (0=السبت، 6=الجمعة) بالتوقيت المحلي */
export function dow(d: Date): number {
  return (d.getDay() + 1) % 7
}

/** أيام الأسبوع الثابت (السبت→الجمعة): offset=0 الأسبوع الحالي، 1 السابق، إلخ */
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
