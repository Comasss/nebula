import { t, locale } from './i18n.svelte.js';

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
  const u = (n, unit) => t(`time.${unit}`, { n });
  if (s < 60) return u(s, 's');
  const m = Math.floor(s / 60);
  if (m < 60) return s % 60 && m < 10 ? `${u(m, 'min')} ${u(s % 60, 's')}` : u(m, 'min');
  const h = Math.floor(m / 60);
  if (h < 24) return m % 60 ? `${u(h, 'h')} ${u(m % 60, 'min')}` : u(h, 'h');
  const d = Math.floor(h / 24);
  return h % 24 ? `${u(d, 'd')} ${u(h % 24, 'h')}` : u(d, 'd');
}

export function ago(ts) {
  if (!ts) return '';
  const d = Date.now() / 1000 - ts;
  if (d < 60) return t('time.justNow');
  if (d < 3600) return t('time.ago', { time: t('time.min', { n: Math.floor(d / 60) }) });
  if (d < 86400) return t('time.ago', { time: t('time.h', { n: Math.floor(d / 3600) }) });
  if (d < 86400 * 7) return t('time.ago', { time: t('time.d', { n: Math.floor(d / 86400) }) });
  return new Date(ts * 1000).toLocaleDateString(locale(), { day: 'numeric', month: 'short', year: 'numeric' });
}

export function dateTime(ts) {
  if (!ts) return '';
  return new Date(ts * 1000).toLocaleString(locale(), { dateStyle: 'medium', timeStyle: 'short' });
}

export function clock(ms) {
  return new Date(ms).toLocaleTimeString(locale(), { hour12: false });
}

export const STATUS = {
  queued: { get label() { return t('status.queued'); }, kind: 'muted', icon: 'clock' },
  downloading: { get label() { return t('status.downloading'); }, kind: 'accent', icon: 'download' },
  paused: { get label() { return t('status.paused'); }, kind: 'muted', icon: 'pause' },
  verifying: { get label() { return t('status.verifying'); }, kind: 'info', icon: 'shield' },
  repairing: { get label() { return t('status.repairing'); }, kind: 'warn', icon: 'wrench' },
  extracting: { get label() { return t('status.extracting'); }, kind: 'teal', icon: 'archive' },
  moving: { get label() { return t('status.moving'); }, kind: 'teal', icon: 'folder' },
  completed: { get label() { return t('status.completed'); }, kind: 'ok', icon: 'check-circle' },
  failed: { get label() { return t('status.failed'); }, kind: 'err', icon: 'x-circle' },
};

/** Nombre de la fase en la que falló una descarga. */
export function stageLabel(stage) {
  return stage ? t(`failedStage.${stage}`) : '';
}

/** Texto normalizado para buscar: minúsculas y sin acentos. */
export function fold(s) {
  return String(s ?? '').toLowerCase().normalize('NFD').replace(/[\u0300-\u036f]/g, '').trim();
}
