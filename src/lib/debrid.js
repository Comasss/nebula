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

/** Servidores NNTP activos, por prioridad. */
export function nntpServers(settings) {
  return (settings?.servers ?? []).filter((s) => s.enabled && s.host?.trim()).sort((a, b) => a.priority - b.priority);
}

export function serverLabel(s) {
  return s?.name?.trim() || s?.host || '';
}

/**
 * Formas de descargar un NZB para el menú "Descargar con": todos los servidores NNTP, cada
 * servidor por separado (si hay varios) y cada servicio debrid. `value` es lo que espera add_job.
 */
export function viaOptions(settings) {
  const servers = nntpServers(settings);
  const out = [];
  if (servers.length) {
    out.push({ value: 'nntp', group: 'nntp', icon: 'server', label: 'Usenet (NNTP)', sub: servers.length > 1 ? t('via.allServers') : serverLabel(servers[0]) });
    if (servers.length > 1) {
      for (const s of servers) {
        out.push({ value: `nntp:${s.id}`, group: 'nntp', icon: 'server', label: serverLabel(s), sub: s.priority === servers[0].priority ? t('via.primary') : t('via.backup', { n: s.priority }), child: true });
      }
    }
  }
  for (const p of usableDebrid(settings)) {
    out.push({ value: p.id, group: 'debrid', icon: 'cloud', label: debridLabel(p), sub: t('add.remoteDownload') });
  }
  return out;
}

/** Nombre corto de un método elegido en el menú (para los avisos). */
export function viaLabel(settings, via) {
  return viaOptions(settings).find((o) => o.value === via)?.label ?? '';
}
