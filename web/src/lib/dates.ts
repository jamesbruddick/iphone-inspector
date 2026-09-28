/** "Today, 7:13 PM", "Yesterday, 9:02 AM", or "Sep 26, 7:13 PM" (with the year once it is not this one). */
export function whenRead(iso: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return '';
  const time = date.toLocaleTimeString('en-US', { hour: 'numeric', minute: '2-digit' });
  const today = new Date();
  const yesterday = new Date(today);
  yesterday.setDate(today.getDate() - 1);
  if (date.toDateString() === today.toDateString()) return `Today, ${time}`;
  if (date.toDateString() === yesterday.toDateString()) return `Yesterday, ${time}`;
  const day = date.toLocaleDateString('en-US', { month: 'short', day: 'numeric', ...(date.getFullYear() !== today.getFullYear() && { year: 'numeric' }) });
  return `${day}, ${time}`;
}

/** "Read at 7:13 PM" for today, otherwise "Read Sep 26, 7:13 PM". */
export function readLabel(iso: string): string {
  const when = whenRead(iso);
  return when.startsWith('Today, ') ? `Read at ${when.slice('Today, '.length)}` : `Read ${when}`;
}
