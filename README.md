# Nebula

Cliente Usenet (NZB) moderno para Windows: alternativa a SABnzbd con interfaz oscura y animada.

- **Motor en Rust** (Tauri 2 + tokio): NNTP con TLS, multiconexión, servidores de respaldo por prioridad, decodificación yEnc con verificación CRC, escritura directa a disco y reanudación tras cerrar la app.
- **Post-procesado integrado**: verificación y reparación PAR2 (par2cmdline-turbo) que descarga solo los bloques de recuperación necesarios, y extracción RAR/7z/ZIP (7-Zip), con barra de progreso, porcentaje y velocidad. Ambas herramientas van **dentro del .exe**.
- **Búsqueda en indexadores Newznab** (NZBGeek, NZBFinder, DrunkenSlug…): varios a la vez o solo los elegidos, con filtro de categoría, orden y agrupación opcional de duplicados (el mismo release resubido a varios indexadores aparece una vez y, si un indexador falla al descargar, se usa el siguiente). Descarga con un clic o guarda el `.nzb` en el PC.
- **Funciones a elegir** en el primer arranque (mínimo una): solo indexadores (guardar `.nzb`), indexadores + NNTP, o solo NNTP.
- **Interfaz Svelte 5**: asistente inicial de credenciales NNTP, pestañas Descargas / Historial / Servidores / Configuración / Registro, arrastrar `.nzb`, pegar enlaces, vigilancia configurable del portapapeles, icono en la bandeja y notificaciones.

## Uso

- Añadir un NZB: botón **Añadir NZB**, arrastrarlo a la ventana, doble clic en un `.nzb` (si la asociación está activada) o pegar un enlace en la barra superior. Nebula descarga el NZB, lo analiza y pregunta dónde guardarlo.
- Contraseñas de archivos: se leen del NZB (`<meta type="password">`) o del nombre (`Nombre {{clave}}`). Si faltan, el historial ofrece **Reintentar extracción**.
- Datos: `%APPDATA%\Nebula` (configuración, cola, historial, registros). Las contraseñas de servidor se cifran con DPAPI.
  La variable de entorno `NEBULA_DATA_DIR` permite usar otra carpeta (modo portable o pruebas).

## Compilar

Requisitos: Rust (MSVC), Node.js y WebView2 (incluido en Windows 11).

```bash
npm install
npx tauri build
```

Genera `src-tauri/target/release/nebula.exe` (portable) y el instalador en `src-tauri/target/release/bundle/nsis/`.

Desarrollo con recarga en caliente:

```bash
npx tauri dev
```

## Pruebas

```bash
cd src-tauri && cargo test
```

`dev/mock_nntp.py` es un servidor NNTP simulado que publica una carpeta como artículos yEnc, genera el `.nzb` y lo sirve por HTTP. Puede simular artículos ausentes (`--missing`), límite de conexiones (`--max-conns`) y velocidad (`--kbps`):

```bash
python dev/mock_nntp.py --src carpeta_a_publicar --nzb prueba.nzb --missing 3 --kbps 1500
```

La API simulada también expone un indexador Newznab en `http://127.0.0.1:8119/api` (API key `--indexer-key`, por defecto `testkey`). Test del cliente de indexadores contra ella o contra uno real:

```bash
cd src-tauri && NEBULA_NEWZNAB=http://127.0.0.1:8119 NEBULA_NEWZNAB_KEY=testkey cargo test live_indexer -- --ignored --nocapture
```

`dev/preview.html` muestra la interfaz en el navegador con la API de Tauri simulada (`npm run dev` y abrir `http://localhost:1420/dev/preview.html`; admite `?setup`, `?feat`, `?idxonly`, `?add`, `?clipboard`).

Para comprobar la conexión con un servidor real sin credenciales (TCP + TLS + saludo):

```bash
cd src-tauri && NEBULA_PROBE=news.servidor.com:563:ssl cargo test probe -- --ignored --nocapture
```

## Licencias de terceros

- 7-Zip © Igor Pavlov — GNU LGPL + restricción unRAR (`src-tauri/bin/7zip-License.txt`).
- par2cmdline-turbo — GNU GPL v2 (https://github.com/animetosho/par2cmdline-turbo).

Se ejecutan como programas independientes; se extraen a `%LOCALAPPDATA%\Nebula\tools` la primera vez.
