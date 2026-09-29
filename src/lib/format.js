const UNITS = ['B', 'KB', 'MB', 'GB', 'TB'];

export function bytes(n, dec = 1) {
  if (!n || n < 0) return '0 B';
  let v = n;
  let i = 0;
  while (v >= 1024 && i < UNITS.length - 1) {
    v /= 1024;
    i++;
  }
  const digits = i === 0 ? 0 : v >= 100 ? 0 : dec;
  return `${v.toFixed(digits)} ${UNITS[i]}`;
}

export function speed(n) {
  return `${bytes(n)}/s`;
}

export function duration(s) {
  if (!s || s <= 0 || !isFinite(s)) return '—';
  s = Math.round(s);
  if (s < 60) return `${s} s`;
  const m = Math.floor(s / 60);
  if (m < 60) return s % 60 && m < 10 ? `${m} min ${s % 60} s` : `${m} min`;
  const h = Math.floor(m / 60);
  if (h < 24) return m % 60 ? `${h} h ${m % 60} min` : `${h} h`;
  const d = Math.floor(h / 24);
  return h % 24 ? `${d} d ${h % 24} h` : `${d} d`;
}

export function ago(ts) {
  if (!ts) return '';
  const d = Date.now() / 1000 - ts;
  if (d < 60) return 'ahora mismo';
  if (d < 3600) return `hace ${Math.floor(d / 60)} min`;
  if (d < 86400) return `hace ${Math.floor(d / 3600)} h`;
  if (d < 86400 * 7) return `hace ${Math.floor(d / 86400)} d`;
  return new Date(ts * 1000).toLocaleDateString('es-ES', { day: 'numeric', month: 'short', year: 'numeric' });
}

export function dateTime(ts) {
  if (!ts) return '';
  return new Date(ts * 1000).toLocaleString('es-ES', { dateStyle: 'medium', timeStyle: 'short' });
}

export function clock(ms) {
  return new Date(ms).toLocaleTimeString('es-ES', { hour12: false });
}

export const STATUS = {
  queued: { label: 'En cola', kind: 'muted', icon: 'clock' },
  downloading: { label: 'Descargando', kind: 'accent', icon: 'download' },
  paused: { label: 'En pausa', kind: 'muted', icon: 'pause' },
  verifying: { label: 'Verificando', kind: 'info', icon: 'shield' },
  repairing: { label: 'Reparando', kind: 'warn', icon: 'wrench' },
  extracting: { label: 'Extrayendo', kind: 'teal', icon: 'archive' },
  moving: { label: 'Moviendo', kind: 'teal', icon: 'folder' },
  completed: { label: 'Completado', kind: 'ok', icon: 'check-circle' },
  failed: { label: 'Fallido', kind: 'err', icon: 'x-circle' },
};

export const STAGES = {
  download: 'descarga',
  verify: 'verificación',
  repair: 'reparación',
  extract: 'extracción',
  move: 'movimiento de archivos',
};
