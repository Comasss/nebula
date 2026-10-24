// Genera los iconos de Android a partir de app-icon.svg, el mismo icono que en Windows.
// Uso: npx -y -p playwright node dev/android-icons.mjs (necesita Chromium de Playwright).
import { chromium } from 'playwright';
import fs from 'fs';
const root = new URL('..', import.meta.url).pathname.replace(/\/$/, '');
const svg = fs.readFileSync(`${root}/app-icon.svg`, 'utf8');
const defs = svg.match(/<defs>[\s\S]*<\/defs>/)[0];
const art = svg.slice(svg.indexOf('</defs>') + 7, svg.lastIndexOf('</svg>')).replace(/<rect[^>]*\/>\s*/g, '');
const wrap = (body) => `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024" width="1024" height="1024">${defs}${body}</svg>`;
// Capa de fondo: degradado y brillo a sangre (el lanzador recorta la forma).
const bgDefs = `<defs><linearGradient id="b2" gradientUnits="userSpaceOnUse" x1="171" y1="171" x2="853" y2="853"><stop offset="0" stop-color="#1d1640"/><stop offset="1" stop-color="#0a0c15"/></linearGradient><radialGradient id="g2" gradientUnits="userSpaceOnUse" cx="512" cy="485" r="410"><stop offset="0" stop-color="#7c5cff" stop-opacity="0.6"/><stop offset="1" stop-color="#7c5cff" stop-opacity="0"/></radialGradient></defs>`;
const bg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024" width="1024" height="1024">${bgDefs}<rect width="1024" height="1024" fill="url(#b2)"/><rect width="1024" height="1024" fill="url(#g2)"/></svg>`;
// Capa delantera: planeta, anillo y flecha, escalados para que el anillo quepa en la zona visible.
const S = 0.7;
const fg = wrap(`<g transform="translate(512 512) scale(${S}) translate(-512 -512)">${art}</g>`);
// Redondo (Android < 8): círculo con el fondo y el dibujo.
const round = wrap(`<clipPath id="c"><circle cx="512" cy="512" r="500"/></clipPath><g clip-path="url(#c)"><rect width="1024" height="1024" fill="url(#bg)"/><rect width="1024" height="1024" fill="url(#glow)"/><g transform="translate(512 512) scale(0.86) translate(-512 -512)">${art}</g></g>`);
const files = { bg, fg, round, legacy: svg };
const dens = { mdpi: 1, hdpi: 1.5, xhdpi: 2, xxhdpi: 3, xxxhdpi: 4 };
const browser = await chromium.launch();
const page = await browser.newPage();
async function render(src, size, out) {
  await page.setViewportSize({ width: size, height: size });
  await page.setContent(`<html><body style="margin:0;background:transparent">${src.replace('width="1024" height="1024"', `width="${size}" height="${size}"`)}</body></html>`);
  await page.screenshot({ path: out, omitBackground: true, clip: { x: 0, y: 0, width: size, height: size } });
}
const res = `${root}/src-tauri/gen/android/app/src/main/res`;
for (const [d, k] of Object.entries(dens)) {
  await render(files.fg, 108 * k, `${res}/mipmap-${d}/ic_launcher_foreground.png`);
  await render(files.bg, 108 * k, `${res}/mipmap-${d}/ic_launcher_background.png`);
  await render(files.legacy, 48 * k, `${res}/mipmap-${d}/ic_launcher.png`);
  await render(files.round, 48 * k, `${res}/mipmap-${d}/ic_launcher_round.png`);
}
await browser.close();
