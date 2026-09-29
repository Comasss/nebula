"""
Servidor NNTP simulado para probar Nebula sin un proveedor real.

Publica (en memoria) todos los archivos de una carpeta como artículos yEnc,
genera el .nzb correspondiente y lo sirve también por HTTP.

Uso:
  python dev/mock_nntp.py --src CARPETA --nzb salida.nzb [--port 1119] [--http 8119]
                          [--user test --password test] [--missing 3] [--kbps 800]
                          [--max-conns 20] [--nzb-password clave]
"""
import argparse
import asyncio
import os
import random
import re
import threading
import time
import zlib
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from xml.sax.saxutils import escape, quoteattr

PART = 500 * 1024
LINE = 128
TABLE = bytes(((i + 42) % 256) for i in range(256))
CRIT = re.compile(rb"[\x00\x0a\x0d=]")


def yenc_lines(data: bytes):
    enc = data.translate(TABLE)
    enc = CRIT.sub(lambda m: b"=" + bytes([(m.group()[0] + 64) % 256]), enc)
    lines, pos, n = [], 0, len(enc)
    while pos < n:
        end = min(pos + LINE, n)
        if enc[end - 1] == 0x3D and end < n:  # no partir una secuencia de escape
            end += 1
        lines.append(enc[pos:end])
        pos = end
    return lines


def build_articles(src):
    store, files = {}, []
    for name in sorted(os.listdir(src)):
        path = os.path.join(src, name)
        if not os.path.isfile(path):
            continue
        data = open(path, "rb").read()
        total = max(1, (len(data) + PART - 1) // PART)
        segs = []
        for i in range(total):
            chunk = data[i * PART:(i + 1) * PART]
            begin = i * PART + 1
            head = f"=ybegin part={i + 1} total={total} line={LINE} size={len(data)} name={name}".encode()
            ypart = f"=ypart begin={begin} end={begin + len(chunk) - 1}".encode()
            yend = f"=yend size={len(chunk)} part={i + 1} pcrc32={zlib.crc32(chunk) & 0xffffffff:08x}".encode()
            lines = [head, ypart] + yenc_lines(chunk) + [yend]
            body = b"\r\n".join((b"." + l if l.startswith(b".") else l) for l in lines) + b"\r\n.\r\n"
            mid = f"{name.replace(' ', '_')}.{i + 1}.{random.randrange(10**9)}@mock.nebula"
            store[mid] = body
            segs.append((i + 1, len(body), mid))
        files.append((name, total, segs))
        print(f"  {name}: {len(data)} bytes, {total} artículos")
    return store, files


def write_nzb(files, out, password=None, title=None):
    now = int(time.time()) - 3600
    parts = ['<?xml version="1.0" encoding="UTF-8"?>',
             '<!DOCTYPE nzb PUBLIC "-//newzBin//DTD NZB 1.1//EN" "http://www.newzbin.com/DTD/nzb/nzb-1.1.dtd">',
             '<nzb xmlns="http://www.newzbin.com/DTD/2003/nzb">', " <head>"]
    if title:
        parts.append(f'  <meta type="title">{escape(title)}</meta>')
    if password:
        parts.append(f'  <meta type="password">{escape(password)}</meta>')
    parts.append(" </head>")
    for idx, (name, total, segs) in enumerate(files):
        subject = f'[{idx + 1}/{len(files)}] - "{name}" yEnc (1/{total})'
        parts.append(f' <file poster="mock@nebula" date="{now}" subject={quoteattr(subject)}>')
        parts.append("  <groups><group>alt.binaries.test</group></groups>")
        parts.append("  <segments>")
        for num, size, mid in segs:
            parts.append(f'   <segment bytes="{size}" number="{num}">{escape(mid)}</segment>')
        parts.append("  </segments>")
        parts.append(" </file>")
    parts.append("</nzb>")
    with open(out, "w", encoding="utf-8") as f:
        f.write("\n".join(parts))


async def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--src", required=True)
    ap.add_argument("--nzb", required=True)
    ap.add_argument("--port", type=int, default=1119)
    ap.add_argument("--http", type=int, default=8119)
    ap.add_argument("--user", default="test")
    ap.add_argument("--password", default="test")
    ap.add_argument("--missing", type=int, default=0, help="artículos de datos que devolverán 430")
    ap.add_argument("--kbps", type=int, default=0, help="límite por conexión en KB/s (0 = sin límite)")
    ap.add_argument("--max-conns", type=int, default=50)
    ap.add_argument("--nzb-password", default=None)
    ap.add_argument("--title", default=None)
    ap.add_argument("--seed", type=int, default=7)
    ap.add_argument("--indexer-key", default="testkey", help="API key de la API Newznab simulada (/api)")
    ap.add_argument("--indexer-title", default=None, help="título del release en la API Newznab")
    args = ap.parse_args()
    random.seed(args.seed)

    print("Generando artículos…")
    store, files = build_articles(args.src)
    write_nzb(files, args.nzb, args.nzb_password, args.title)
    data_ids = [mid for (name, _, segs) in files if not name.lower().endswith(".par2") for (_, _, mid) in segs]
    missing = set(random.sample(data_ids, min(args.missing, len(data_ids))))
    print(f"NZB: {args.nzb} ({len(store)} artículos, {len(missing)} ausentes)")

    # HTTP para probar el flujo "pegar enlace"
    nzb_dir = os.path.dirname(os.path.abspath(args.nzb))

    class H(SimpleHTTPRequestHandler):
        def __init__(self, *a, **k):
            super().__init__(*a, directory=nzb_dir, **k)

        def log_message(self, *a):
            pass

        # API Newznab mínima: t=caps, t=search y t=get
        def do_GET(self):
            from urllib.parse import urlparse, parse_qs
            u = urlparse(self.path)
            if not u.path.rstrip("/").endswith("/api"):
                return super().do_GET()
            q = {k: v[0] for k, v in parse_qs(u.query).items()}
            t = q.get("t", "")
            base = f"http://127.0.0.1:{args.http}/api"
            title = args.indexer_title or os.path.splitext(os.path.basename(args.nzb))[0]
            if t == "caps":
                body = ('<?xml version="1.0" encoding="UTF-8"?><caps><server title="Mock Newznab"/>'
                        '<limits max="100" default="50"/><searching><search available="yes"/></searching></caps>')
                return self._send(body, "application/xml")
            if q.get("apikey") != args.indexer_key:
                return self._send('<?xml version="1.0" encoding="UTF-8"?><error code="100" description="Incorrect user credentials"/>', "application/xml")
            if t == "get":
                data = open(args.nzb, "rb").read()
                return self._send(data, "application/x-nzb", {"Content-Disposition": f'attachment; filename="{title}.nzb"'})
            if t == "search":
                words = [w for w in q.get("q", "").lower().split() if w]
                match = all(w in title.lower() for w in words)
                items = ""
                if match and q.get("offset", "0") == "0":
                    size = sum(os.path.getsize(os.path.join(args.src, f)) for f in os.listdir(args.src))
                    date = time.strftime("%a, %d %b %Y %H:%M:%S +0000", time.gmtime(time.time() - 7200))
                    link = escape(f"{base}?t=get&id=1&apikey={args.indexer_key}")
                    items = (f"<item><title>{escape(title)}</title><guid isPermaLink=\"true\">{base}/details/1</guid>"
                             f"<link>{link}</link><pubDate>{date}</pubDate><category>Movies &gt; HD</category>"
                             f'<enclosure url="{link}" length="{size}" type="application/x-nzb"/>'
                             f'<newznab:attr name="category" value="2000"/><newznab:attr name="category" value="2040"/>'
                             f'<newznab:attr name="size" value="{size}"/><newznab:attr name="grabs" value="7"/>'
                             f'<newznab:attr name="usenetdate" value="{date}"/></item>')
                total = 1 if items else 0
                body = ('<?xml version="1.0" encoding="UTF-8"?><rss version="2.0" '
                        'xmlns:newznab="http://www.newznab.com/DTD/2010/feeds/attributes/"><channel>'
                        f'<newznab:response offset="0" total="{total}"/>{items}</channel></rss>')
                return self._send(body, "application/rss+xml")
            return self._send('<?xml version="1.0"?><error code="202" description="No such function"/>', "application/xml")

        def _send(self, body, ctype, headers=None):
            data = body.encode() if isinstance(body, str) else body
            self.send_response(200)
            self.send_header("Content-Type", ctype)
            self.send_header("Content-Length", str(len(data)))
            for k, v in (headers or {}).items():
                self.send_header(k, v)
            self.end_headers()
            self.wfile.write(data)

    httpd = ThreadingHTTPServer(("127.0.0.1", args.http), H)
    threading.Thread(target=httpd.serve_forever, daemon=True).start()
    print(f"HTTP: http://127.0.0.1:{args.http}/{os.path.basename(args.nzb)}")

    active = 0

    async def handle(reader, writer):
        nonlocal active
        if active >= args.max_conns:
            writer.write(b"502 Too many connections from your IP\r\n")
            await writer.drain()
            writer.close()
            return
        active += 1
        authed = False
        user = None
        try:
            writer.write(b"200 Mock NNTP listo\r\n")
            await writer.drain()
            while True:
                line = await reader.readline()
                if not line:
                    break
                cmd = line.decode("latin-1").strip()
                up = cmd.upper()
                if up.startswith("AUTHINFO USER"):
                    user = cmd[14:].strip()
                    writer.write(b"381 Password required\r\n")
                elif up.startswith("AUTHINFO PASS"):
                    pw = cmd[14:].strip()
                    if user == args.user and pw == args.password:
                        authed = True
                        writer.write(b"281 Authentication accepted\r\n")
                    else:
                        writer.write(b"481 Authentication failed\r\n")
                elif up.startswith("BODY"):
                    if not authed:
                        writer.write(b"480 Authentication required\r\n")
                    else:
                        mid = cmd[4:].strip().strip("<>")
                        body = store.get(mid)
                        if body is None or mid in missing:
                            writer.write(b"430 No such article\r\n")
                        else:
                            writer.write(f"222 0 <{mid}>\r\n".encode())
                            if args.kbps:
                                step = 32 * 1024
                                for i in range(0, len(body), step):
                                    writer.write(body[i:i + step])
                                    await writer.drain()
                                    await asyncio.sleep(step / (args.kbps * 1024))
                            else:
                                writer.write(body)
                elif up.startswith("DATE"):
                    writer.write(time.strftime("111 %Y%m%d%H%M%S\r\n", time.gmtime()).encode())
                elif up.startswith("QUIT"):
                    writer.write(b"205 Bye\r\n")
                    await writer.drain()
                    break
                else:
                    writer.write(b"500 Unknown command\r\n")
                await writer.drain()
        except (ConnectionResetError, BrokenPipeError):
            pass
        finally:
            active -= 1
            writer.close()

    server = await asyncio.start_server(handle, "127.0.0.1", args.port)
    print(f"NNTP: 127.0.0.1:{args.port} (usuario {args.user} / {args.password})")
    async with server:
        await server.serve_forever()


if __name__ == "__main__":
    asyncio.run(main())
