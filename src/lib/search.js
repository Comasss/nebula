// Lógica de búsqueda en indexadores: categorías, presets, agrupación de duplicados y orden.

export const CATEGORIES = [
  { id: '', label: 'Todas las categorías' },
  { id: '2000', label: 'Películas' },
  { id: '5000', label: 'Series' },
  { id: '3000', label: 'Música' },
  { id: '7000', label: 'Libros' },
  { id: '4000', label: 'PC y software' },
  { id: '1000', label: 'Consolas' },
  { id: '8000', label: 'Otros' },
  { id: '6000', label: 'Adultos' },
];

const PARENT = { 1000: 'Consolas', 2000: 'Películas', 3000: 'Música', 4000: 'PC', 5000: 'Series', 6000: 'Adultos', 7000: 'Libros', 8000: 'Otros' };

export function categoryLabel(item) {
  const id = (item.categoryIds ?? []).find((c) => c >= 1000);
  if (id) return PARENT[Math.floor(id / 1000) * 1000] ?? item.category;
  return (item.category ?? '').split('>')[0].trim();
}

export const SORTS = [
  { id: 'date', label: 'Más recientes' },
  { id: 'date-asc', label: 'Más antiguos' },
  { id: 'size-desc', label: 'Mayor tamaño' },
  { id: 'size-asc', label: 'Menor tamaño' },
  { id: 'grabs', label: 'Más descargados' },
  { id: 'name', label: 'Nombre' },
];

export const PRESETS = [
  { name: 'NZBGeek', url: 'https://api.nzbgeek.info' },
  { name: 'NZBFinder', url: 'https://nzbfinder.ws' },
  { name: 'DrunkenSlug', url: 'https://drunkenslug.com' },
  { name: 'NZBPlanet', url: 'https://api.nzbplanet.net' },
  { name: 'DOGnzb', url: 'https://api.dognzb.cr' },
  { name: 'altHUB', url: 'https://api.althub.co.za' },
  { name: 'NinjaCentral', url: 'https://ninjacentral.co.za' },
  { name: 'NZB.su', url: 'https://api.nzb.su' },
];

export function blankIndexer() {
  return { id: crypto.randomUUID().replaceAll('-', ''), name: '', url: '', apiKey: '', apiKeyEnc: '', apiPath: '/api', enabled: true };
}

// Sufijos que añaden algunos indexadores o reposters al mismo release.
const REPOST_TAG = /[-._ ](obfuscated|postbot|xpost|repost|scrambled|asrequested|buymore|rakuv\d*|whiterev)$/i;

/** Nombre normalizado para detectar el mismo release publicado en varios indexadores. */
export function normalizeTitle(title) {
  let s = String(title).toLowerCase().trim();
  s = s.replace(/\.(nzb|mkv|mp4|avi)$/, '').replace(/\s*\[[^\]]*\]\s*$/, '');
  let prev;
  do {
    prev = s;
    s = s.replace(REPOST_TAG, '').trim();
  } while (s !== prev);
  return s.replace(/[^a-z0-9]+/g, '');
}

function sameSize(a, b) {
  if (!a || !b) return true;
  return Math.abs(a - b) / Math.max(a, b) < 0.03;
}

/**
 * Agrupa los resultados. Con `dedup`, el mismo release (nombre normalizado y tamaño ±3 %)
 * de varios indexadores queda en un solo grupo con todas sus fuentes, ordenadas por la
 * prioridad del indexador (orden en la lista) y por número de descargas.
 */
export function groupResults(items, dedup, indexerOrder = []) {
  const rank = (id) => {
    const i = indexerOrder.indexOf(id);
    return i < 0 ? 999 : i;
  };
  const groups = [];
  const byTitle = new Map();
  const usedKeys = new Set();
  for (const it of items) {
    let g = null;
    const norm = normalizeTitle(it.title);
    if (dedup) {
      const same = byTitle.get(norm) ?? [];
      g = same.find((c) => sameSize(c.size, it.size)) ?? null;
      if (!g) {
        let key = `${norm}#${same.length}`;
        g = { key, sources: [], size: it.size };
        same.push(g);
        byTitle.set(norm, same);
        groups.push(g);
      }
    } else {
      let key = `${it.indexerId}|${it.link}`;
      while (usedKeys.has(key)) key += '+';
      usedKeys.add(key);
      g = { key, sources: [], size: it.size };
      groups.push(g);
    }
    g.sources.push(it);
  }
  for (const g of groups) {
    g.sources.sort((a, b) => rank(a.indexerId) - rank(b.indexerId) || (b.grabs ?? 0) - (a.grabs ?? 0));
    const best = g.sources[0];
    g.title = best.title;
    g.size = best.size || Math.max(...g.sources.map((s) => s.size));
    g.date = Math.max(...g.sources.map((s) => s.date || 0));
    const grabs = g.sources.map((s) => s.grabs).filter((x) => x != null);
    g.grabs = grabs.length ? grabs.reduce((a, b) => a + b, 0) : null;
    g.category = categoryLabel(best);
    g.password = g.sources.some((s) => s.password > 0);
    g.indexers = [...new Set(g.sources.map((s) => s.indexerName))];
    g.details = g.sources.find((s) => s.details)?.details ?? null;
  }
  return groups;
}

export function sortGroups(groups, sort) {
  const list = [...groups];
  const by = {
    date: (a, b) => b.date - a.date,
    'date-asc': (a, b) => a.date - b.date,
    'size-desc': (a, b) => b.size - a.size,
    'size-asc': (a, b) => a.size - b.size,
    grabs: (a, b) => (b.grabs ?? -1) - (a.grabs ?? -1),
    name: (a, b) => a.title.localeCompare(b.title),
  }[sort] ?? ((a, b) => b.date - a.date);
  return list.sort(by);
}
