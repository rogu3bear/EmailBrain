const SHORT_DATE_TIME_FORMAT = new Intl.DateTimeFormat(undefined, {
  year: 'numeric',
  month: 'short',
  day: 'numeric',
  hour: '2-digit',
  minute: '2-digit',
  timeZoneName: 'short',
});

const LONG_DATE_TIME_FORMAT = new Intl.DateTimeFormat(undefined, {
  weekday: 'long',
  year: 'numeric',
  month: 'long',
  day: 'numeric',
  hour: '2-digit',
  minute: '2-digit',
  timeZoneName: 'short',
});

function parseDate(dateString: string): Date | null {
  const date = new Date(dateString);
  return Number.isNaN(date.getTime()) ? null : date;
}

export function formatShortDateTime(dateString: string): string {
  const date = parseDate(dateString);
  return date ? SHORT_DATE_TIME_FORMAT.format(date) : 'Unknown date';
}

export function formatLongDateTime(dateString: string): string {
  const date = parseDate(dateString);
  return date ? LONG_DATE_TIME_FORMAT.format(date) : 'Unknown date';
}

export function formatEndpointLabel(endpoint: string): string {
  try {
    return new URL(endpoint).host;
  } catch {
    return endpoint;
  }
}
