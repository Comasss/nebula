import { t } from './i18n.svelte.js';

// Servicios debrid que descargan NZB por API (TorBox Pro, Premiumize).

export const DEBRID_KINDS = [
  {
    id: 'torbox',
    name: 'TorBox',
    keyUrl: 'https://torbox.app/settings',
    get keyHelp() {
      return t('debrid.keyHelpTorbox');
    },
    cache: true,
  },
  {
    id: 'premiumize',
    name: 'Premiumize',
    keyUrl: 'https://www.premiumize.me/account',
    get keyHelp() {
      return t('debrid.keyHelpPremiumize');
    },
    cache: false,
  },
];

export function kindInfo(kind) {
  return DEBRID_KINDS.find((k) => k.id === kind) ?? DEBRID_KINDS[0];
}

export function debridLabel(p) {
  return p?.name?.trim() || kindInfo(p?.kind).name;
}

export function blankDebrid(kind = 'torbox') {
  return { id: crypto.randomUUID().replaceAll('-', ''), kind, name: '', apiKey: '', apiKeyEnc: '', enabled: true };
}

/** Servicios activos y con API key, en orden de preferencia. */
export function usableDebrid(settings) {
  return (settings?.debrid ?? []).filter((p) => p.enabled && p.apiKey?.trim());
}

/** ¿Hay algún servidor NNTP activo? */
export function hasNntp(settings) {
  return (settings?.servers ?? []).some((s) => s.enabled && s.host?.trim());
}

/** Método por defecto para un NZB nuevo: 'nntp' o el id del servicio debrid. */
export function defaultVia(settings) {
  const usable = usableDebrid(settings);
  if (usable.length && (settings.downloadVia === 'debrid' || !hasNntp(settings))) return usable[0].id;
  return 'nntp';
}
