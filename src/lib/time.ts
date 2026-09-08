export function relativeTime(isoDate: string, locale: string, now = Date.now()): string {
  const formatter = new Intl.RelativeTimeFormat(locale, { numeric: "auto" });
  const seconds = Math.round((new Date(isoDate).getTime() - now) / 1000);
  if (Math.abs(seconds) < 60) return formatter.format(seconds, "second");
  const minutes = Math.round(seconds / 60);
  if (Math.abs(minutes) < 60) return formatter.format(minutes, "minute");
  const hours = Math.round(minutes / 60);
  if (Math.abs(hours) < 24) return formatter.format(hours, "hour");
  return formatter.format(Math.round(hours / 24), "day");
}
