# Nebula

Cliente Usenet (NZB) moderno para Windows: alternativa a SABnzbd con interfaz oscura y animada.

- **Motor en Rust** (Tauri 2 + tokio): NNTP con TLS, multiconexión, servidores de respaldo por prioridad, decodificación yEnc con verificación CRC, escritura directa a disco y reanudación tras cerrar la app.
- **Post-procesado integrado**: verificación y reparación PAR2 (par2cmdline-turbo) que descarga solo los bloques de recuperación necesarios, y extracción RAR/7z/ZIP (7-Zip), con barra de progreso, porcentaje y velocidad. Ambas herramientas van **dentro del .exe**.
- **Búsqueda en indexadores Newznab** (NZBGeek, NZBFinder, DrunkenSlug…): varios a la vez o solo los elegidos, con filtro de categoría, orden y agrupación opcional de duplicados (el mismo release resubido a varios indexadores aparece una vez y, si un indexador falla al descargar, se usa el siguiente). Descarga con un clic o guarda el `.nzb` en el PC.
- **Descarga a través de servicios debrid** (TorBox Pro y Premiumize): Nebula les envía el NZB, ellos lo descargan (y reparan/extraen) en sus servidores y Nebula baja los archivos sueltos por HTTPS con varias conexiones por rangos, al mismo sitio y con el mismo formato que por NNTP (nunca un `.zip`). Se elige el método por defecto y, en cada NZB, NNTP o el servicio que quieras.
- **Funciones a elegir** en el primer arranque (mínimo una): solo indexadores (guardar `.nzb`), indexadores + NNTP, o solo NNTP.
- **Interfaz Svelte 5**: asistente inicial de credenciales NNTP, pestañas Descargas / Historial / Servidores / Configuración / Registro, arrastrar `.nzb`, pegar enlaces, vigilancia configurable del portapapeles, icono en la bandeja y notificaciones.

## Descarga

En [Releases](../../releases) hay dos opciones para Windows 10/11 x64:

- `Nebula_<versión>_x64-setup.exe`: instalador (acceso directo, desinstalador y asociación de `.nzb`).
- `Nebula_<versión>_x64_portable.exe`: un solo ejecutable, sin instalación.

Ambos necesitan WebView2, que viene de serie en Windows 11 (el instalador lo descarga si falta).

## Uso

- Añadir un NZB: botón **Añadir NZB**, arrastrarlo a la ventana, doble clic en un `.nzb` (si la asociación está activada) o pegar un enlace en la barra superior. Nebula descarga el NZB, lo analiza y pregunta dónde guardarlo.
- Contraseñas de archivos: se leen del NZB (`<meta type="password">`) o del nombre (`Nombre {{clave}}`). Si faltan, el historial ofrece **Reintentar extracción**.
- Descargas debrid: en la pestaña **Debrid** añade tu API key de TorBox (la descarga de NZB requiere el plan Pro) o de Premiumize (cuenta premium) y pulsa **Probar**. En **Configuración → NNTP / Debrid** eliges el método por defecto:
  - **Usenet (NNTP)**: tus servidores. Al añadir un NZB puedes elegir igualmente un servicio debrid en «Descargar con».
  - **Debrid**: se usa el primer servicio de la lista; si falla se prueba el siguiente y, si lo activas, NNTP como último recurso. Si no tienes servidores NNTP, se usa debrid automáticamente.

  Cada descarga pasa por tres fases: envío del NZB, descarga en el servicio (se ve su progreso y velocidad) y descarga al PC (por defecto 8 conexiones, fragmentos de 32 MB, reanudable tras cerrar Nebula). Después se verifica/extrae con el post-procesado habitual si hace falta y, opcionalmente, se borra del servicio. Al añadir un NZB, si TorBox ya lo tiene en caché se marca como instantáneo (⚡).
- Datos: `%APPDATA%\Nebula` (configuración, cola, historial, registros). Las contraseñas de servidor se cifran con DPAPI.
  La variable de entorno `NEBULA_DATA_DIR` permite usar otra carpeta (modo portable o pruebas).

## Compilación automática (GitHub Actions)

`.github/workflows/build.yml` compila Nebula en `windows-latest` en cada push y pull request:

- Ejecuta los tests del motor (`cargo test`) y compila la interfaz.
- Genera el instalador y el portable y los sube como artefacto de la ejecución (pestaña **Actions** → la ejecución → **Artifacts**).
- En cada push a `main` publica la **Release** `vX.Y.Z` de la versión de `src-tauri/tauri.conf.json` con `Nebula_<versión>_x64-setup.exe`, `Nebula_<versión>_x64_portable.exe` y `SHA256SUMS.txt`:
  - Si esa versión aún no tiene Release, la crea.
  - Si la creó este mismo workflow, la actualiza con la nueva compilación (y mueve la etiqueta al último commit).
  - Si la publicaste a mano, no la toca.

  Para publicar una versión nueva, sube el número en `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` y `package.json` (por ejemplo, `npm version 1.2.0 --no-git-tag-version` para `package.json`). También se puede publicar subiendo una etiqueta `vX.Y.Z` con esa misma versión.

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

El mismo servidor simula las API de **TorBox** y **Premiumize** (reciben el NZB, «descargan» durante `--debrid-secs` segundos y sirven los archivos de `--src` con rangos). Para probar la descarga debrid sin cuenta, arranca Nebula apuntando a ellas y usa la API key `--debrid-key` (por defecto `debridkey`); `--debrid-kbps` limita la velocidad y `--debrid-fail` simula una descarga fallida:

```powershell
$env:NEBULA_TORBOX_URL = "http://127.0.0.1:8119/torbox/v1/api"
$env:NEBULA_PREMIUMIZE_URL = "http://127.0.0.1:8119/premiumize/api"
$env:NEBULA_DATA_DIR = "$env:TEMP\nebula-pruebas"
npx tauri dev
```

`dev/preview.html` muestra la interfaz en el navegador con la API de Tauri simulada (`npm run dev` y abrir `http://localhost:1420/dev/preview.html`; admite `?setup`, `?feat`, `?idxonly`, `?add`, `?clipboard`, `?nodebrid`, `?via=debrid`).

Para comprobar la conexión con un servidor real sin credenciales (TCP + TLS + saludo):

```bash
cd src-tauri && NEBULA_PROBE=news.servidor.com:563:ssl cargo test probe -- --ignored --nocapture
```

## Licencias de terceros

- 7-Zip © Igor Pavlov — GNU LGPL + restricción unRAR (`src-tauri/bin/7zip-License.txt`).
- par2cmdline-turbo — GNU GPL v2 (https://github.com/animetosho/par2cmdline-turbo).

Se ejecutan como programas independientes; se extraen a `%LOCALAPPDATA%\Nebula\tools` la primera vez.
