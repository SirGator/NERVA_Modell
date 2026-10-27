#!/usr/bin/env python3
"""Local dashboard API. Python standard library only; also packaged in Messungen/GUI."""
from __future__ import annotations
import argparse, csv, io, json, math, mimetypes, os, secrets, signal, struct, subprocess, threading, time, webbrowser, zlib
from collections import deque
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import parse_qs, quote, unquote, urlsplit

MAX_JSON = 2 * 1024 * 1024
DENIED = {'.git', '.aws', '.agents', '.codex', 'node_modules', '__pycache__'}

def read_json(path, default=None, max_size=MAX_JSON):
    try:
        if path.stat().st_size > max_size: return default
        return json.loads(path.read_text())
    except (OSError, ValueError): return default

def atomic_json(path, data):
    temporary = path.with_name(path.name + '.tmp')
    temporary.write_text(json.dumps(data, allow_nan=False))
    temporary.replace(path)

def safe_path(root, value):
    p = (root / value).resolve()
    if not p.is_relative_to(root) or any(part in DENIED or part.startswith('.') for part in Path(value).parts):
        raise ValueError('Pfad außerhalb des Datenverzeichnisses oder nicht freigegeben.')
    return p

def numeric(value):
    try:
        number = float(value)
        return number if math.isfinite(number) else None
    except (ValueError, TypeError): return None

def csv_preview(path, offset=0, limit=100, search=''):
    rows=[]; scanned=0; more=False
    with path.open(newline='', encoding='utf-8-sig', errors='replace') as f:
        reader=csv.DictReader(f); columns=reader.fieldnames or []
        if len(columns)>256: raise ValueError('Zu viele Spalten für die Vorschau.')
        for index, row in enumerate(reader):
            if index < offset: continue
            scanned=index+1
            if scanned-offset > 50_000:
                more=True;break
            if search and search.casefold() not in ' '.join(str(v) for v in row.values()).casefold():continue
            if len(rows) >= limit:
                more=True;break
            rows.append({k:(v[:4096] if isinstance(v,str) else v) for k,v in row.items() if k is not None})
    return {'kind':'csv','columns':columns,'rows':rows,'offset':offset,'next_offset':scanned-1 if more else scanned,'has_more':more,'scanned':max(0,scanned-offset),'limited':scanned-offset>50_000}

def ppm_png(path):
    raw=path.read_bytes();cursor=0;tokens=[]
    while len(tokens)<4:
        while cursor<len(raw) and raw[cursor] in b' \t\r\n':cursor+=1
        if cursor>=len(raw):raise ValueError('Unvollständiges Bild.')
        if raw[cursor]==35:
            cursor=raw.index(b'\n',cursor)+1;continue
        end=cursor
        while end<len(raw) and raw[end] not in b' \t\r\n':end+=1
        tokens.append(raw[cursor:end]);cursor=end
    if tokens[0]!=b'P6' or tokens[3]!=b'255':raise ValueError('Nicht unterstütztes PPM-Format.')
    width,height=map(int,tokens[1:3])
    if width<1 or height<1 or width*height>16_777_216:raise ValueError('Ungültige Bildgröße.')
    cursor+=2 if raw[cursor:cursor+2]==b'\r\n' else 1
    pixels=raw[cursor:]
    if len(pixels)!=width*height*3:raise ValueError('Unvollständiger Bildinhalt.')
    def chunk(kind,data):return struct.pack('>I',len(data))+kind+data+struct.pack('>I',zlib.crc32(kind+data)&0xffffffff)
    raster=b''.join(b'\0'+pixels[y*width*3:(y+1)*width*3] for y in range(height))
    return b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',width,height,8,2,0,0,0))+chunk(b'IDAT',zlib.compress(raster,3))+chunk(b'IEND',b'')

class Dataset:
    def __init__(self, root):self.root=root.resolve()
    def runs(self):
        result=[]
        for bucket in ('Experimente','Modell'):
            parent=self.root/bucket
            if not parent.exists():continue
            for p in parent.iterdir():
                if not p.is_dir() or p.name.startswith('.'):continue
                manifest=read_json(p/'manifest.json',{})
                meta=read_json(p/'report.json',{})
                seeds=manifest.get('seeds',[]) or [r['seed'] for r in manifest.get('references',[]) if 'seed' in r] or ([meta['seed']] if 'seed' in meta else [])
                kind='Bestätigungsreihe' if 'confirm' in p.name else 'Pilot' if 'pilot' in p.name else 'Live-Lauf' if bucket=='Modell' else 'Diagnose' if any(t in p.name for t in ('audit','diagnostic','control')) else 'Messlauf'
                if p.name in ('interrupted','agency-ten-seeds') or meta.get('interrupted'):kind='Unterbrochen'
                fixed=manifest.get('fixed',{})
                conditions=manifest.get('conditions') or manifest.get('cases') or (fixed.get('conditions',[]) if isinstance(fixed,dict) else [])
                report=next((q.name for q in sorted(p.glob('*summary.md'))),None)
                if not report and (p/'verification.md').exists():report='verification.md'
                result.append({'id':p.relative_to(self.root).as_posix(),'name':p.name,'bucket':bucket,'kind':kind,'seeds':seeds,'conditions':len(conditions),'report':report,'modified':p.stat().st_mtime,'protocol':manifest.get('protocol','')})
        return sorted(result,key=lambda r:(r['name']=='m8-confirm',r['modified']),reverse=True)
    def run(self, name):
        if not name or len(Path(name).parts)!=2 or Path(name).parts[0] not in ('Experimente','Modell'):raise ValueError('Ungültige Messreihe.')
        p=safe_path(self.root,name)
        if not p.is_dir():raise FileNotFoundError(name)
        return p
    def detail(self,name):
        p=self.run(name)
        preferred=['summary.csv','motor-summary.csv','sensor-summary.csv','mapping-summary.csv']
        tables=[]
        for filename in preferred:
            if (p/filename).exists():tables.append({'name':filename,**csv_preview(p/filename,limit=200)})
        report=next((q.name for q in sorted(p.glob('*summary.md'))),None)
        manifest=read_json(p/'manifest.json',{})
        report_meta=read_json(p/'report.json',{})
        files=[{'name':f.name,'directory':f.is_dir(),'size':f.stat().st_size if f.is_file() else None} for f in p.iterdir() if not f.name.startswith('.')]
        return {'id':name,'manifest':manifest,'meta':report_meta,'tables':tables,'report':report,'files':sorted(files,key=lambda x:(not x['directory'],x['name']))}
    def files(self,run,prefix=''):
        parent=self.run(run);p=safe_path(parent,prefix)
        if not p.is_dir():raise ValueError('Kein Verzeichnis.')
        files=[]
        for f in p.iterdir():
            if f.name.startswith('.') or f.name in DENIED:continue
            if not f.resolve().is_relative_to(parent):continue
            files.append({'name':f.name,'path':f.relative_to(parent).as_posix(),'directory':f.is_dir(),'size':f.stat().st_size if f.is_file() else None})
        return sorted(files,key=lambda x:(not x['directory'],x['name']))
    def preview(self,value,offset=0,search=''):
        p=safe_path(self.root,value)
        if not p.is_file():raise FileNotFoundError(value)
        with p.open('rb') as f:prefix=f.read(64)
        if prefix.startswith(b'version https://git-lfs.github.com/spec/v1'):
            return {'kind':'text','text':'Diese Datei ist noch ein Git-LFS-Verweis. Bitte git lfs pull ausführen, um die Messdaten zu laden.','truncated':False}
        if p.suffix=='.csv':return csv_preview(p,offset,100,search)
        if p.suffix.lower() in ('.ppm','.png','.jpg','.jpeg','.bmp'):
            return {'kind':'image','path':value,'size':p.stat().st_size}
        with p.open('rb') as f:raw=f.read(256*1024+1)
        if raw.startswith(b'version https://git-lfs.github.com/spec/v1'):
            return {'kind':'text','text':'Diese Datei ist noch ein Git-LFS-Verweis. Bitte git lfs pull ausführen, um die Messdaten zu laden.','truncated':False}
        text=raw[:256*1024].decode('utf-8',errors='replace')
        if p.suffix=='.json' and len(raw)<=256*1024:
            try:text=json.dumps(json.loads(text),ensure_ascii=False,indent=2)
            except ValueError:pass
        return {'kind':'markdown' if p.suffix=='.md' else 'text','text':text,'truncated':len(raw)>256*1024,'size':p.stat().st_size}

class Live:
    def __init__(self,root,model_script=None):
        self.root=root;self.script=model_script;self.process=None;self.run=None;self.logs=deque(maxlen=120);self.lock=threading.RLock();self.flags={};self.started=0
    def state(self):
        with self.lock:
            running=self.process is not None and self.process.poll() is None
            if not self.run:return {'status':'ready','managed':False,'logs':list(self.logs)}
            state=read_json(self.run/'dashboard.json',{}) or {}
            if not running and self.process and self.process.returncode:
                state['status']='error';state['error']=f'Modell beendet (Code {self.process.returncode}). Details im Laufprotokoll.'
            elif not running and state.get('status') not in ('completed','stopped'):state['status']='completed'
            elif running and not state:state={'status':'starting'}
            state.update({'managed':running,'run':self.run.relative_to(self.root).as_posix(),'logs':list(self.logs),'paused':self.flags.get('paused',False),'started_at':self.started})
            return state
    def start(self,config):
        with self.lock:
            if not self.script:raise ValueError('Dieses Archiv startet keine Modelle.')
            if self.process and self.process.poll() is None:raise ValueError('Ein Modell läuft bereits.')
            def integer(key,default,low,high):
                value=config.get(key,default)
                if key=='seed' and isinstance(value,str) and value.isdecimal():value=int(value)
                if type(value) is not int or not low<=value<=high:raise ValueError(f'Ungültiger Wert für {key}.')
                return value
            steps=integer('steps',1000,1,10_000);seed=integer('seed',42,0,2**64-1);processing=integer('processing',256,1,4096)
            keyboard=config.get('keyboard','full')
            if keyboard not in ('full','wasd','ad'):raise ValueError('Ungültige Tastatur.')
            self.root.joinpath('Modell').mkdir(exist_ok=True)
            self.run=self.root/'Modell'/f'live-{time.time_ns()}-seed-{seed}'
            self.logs.clear();self.flags={};self.started=time.time()
            args=[str(self.script),'demo','--dashboard','--steps',str(steps),'--seed',str(seed),'--processing',str(processing),'--keyboard',keyboard,'--output',str(self.run)]
            for key,flag in [('learning','--frozen'),('development','--fixed-topology'),('homeostasis','--no-homeostasis'),('feedback','--no-feedback')]:
                value=config.get(key,True)
                if type(value) is not bool:raise ValueError(f'Ungültiger Schalter {key}.')
                if not value:args.append(flag)
            self.process=subprocess.Popen(args,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True,bufsize=1,start_new_session=True)
            threading.Thread(target=self._drain,args=(self.process,self.run),daemon=True).start()
            return self.state()
    def _drain(self,process,run):
        # Continuously drain output: a full pipe must never stall the model.
        log=None
        try:
            for line in process.stdout:
                self.logs.append(line.rstrip()[:3000])
                if log is None and run.exists():log=(run/'dashboard.log').open('a')
                if log:log.write(line);log.flush()
            process.wait()
        finally:
            process.stdout.close()
            if log:log.close()
    def control(self,action):
        with self.lock:
            if not self.process or self.process.poll() is not None:raise ValueError('Kein laufendes Modell.')
            if action not in ('pause','resume','stop','freeze'):raise ValueError('Unbekannte Aktion.')
            if not self.run.exists():
                if action=='stop':os.killpg(self.process.pid,signal.SIGTERM);return self.state()
                raise ValueError('Das Modell wird noch vorbereitet.')
            if action in ('pause','resume'):self.flags['paused']=action=='pause'
            elif action=='freeze':self.flags['freeze']=True
            else:self.flags['stop']=True;self.flags['paused']=False
            atomic_json(self.run/'dashboard-control.json',self.flags)
            if action=='stop':
                def terminate_if_stuck(process):
                    try:process.wait(timeout=10)
                    except subprocess.TimeoutExpired:
                        if process.poll() is None:os.killpg(process.pid,signal.SIGTERM)
                threading.Thread(target=terminate_if_stuck,args=(self.process,),daemon=True).start()
            return self.state()
    def close(self):
        if self.process and self.process.poll() is None:
            self.control('stop')
            try:self.process.wait(timeout=12)
            except subprocess.TimeoutExpired:os.killpg(self.process.pid,signal.SIGKILL)

class Server(ThreadingHTTPServer):
    daemon_threads=True
    def __init__(self,address,root,static,script=None):
        super().__init__(address,Handler);self.dataset=Dataset(root);self.static=static;self.live=Live(root,script);self.token=secrets.token_urlsafe(32)

class Handler(BaseHTTPRequestHandler):
    def log_message(self,fmt,*args):
        if args and ' 200 ' not in str(args[0]):pass
    def trusted(self):
        host=urlsplit('http://'+self.headers.get('Host','')).hostname
        if host not in ('127.0.0.1','localhost','::1'):raise ValueError('Nur lokaler Zugriff erlaubt.')
        origin=self.headers.get('Origin')
        if origin and urlsplit(origin).netloc != self.headers.get('Host'):raise ValueError('Anfrage von fremdem Ursprung abgewiesen.')
    def send(self,data,content_type='application/json',status=200):
        body=json.dumps(data,ensure_ascii=False,allow_nan=False).encode() if content_type=='application/json' else data
        self.send_response(status);self.send_header('Content-Type',content_type);self.send_header('Content-Length',len(body));self.send_header('Cache-Control','no-store');self.send_header('X-Content-Type-Options','nosniff');self.end_headers()
        try:self.wfile.write(body)
        except (BrokenPipeError,ConnectionResetError):pass
    def do_GET(self):
        try:
            self.trusted();url=urlsplit(self.path);q=parse_qs(url.query);get=lambda key,default='':q.get(key,[default])[0];ds=self.server.dataset
            if url.path=='/api/config':return self.send({'live':bool(self.server.live.script),'token':self.server.token,'root_name':ds.root.name,'poll_ms':1000})
            if url.path=='/api/runs':return self.send(ds.runs())
            if url.path=='/api/run':return self.send(ds.detail(get('id')))
            if url.path=='/api/files':return self.send(ds.files(get('run'),get('prefix')))
            if url.path=='/api/preview':return self.send(ds.preview(get('path'),max(0,min(int(get('offset','0')),10_000_000)),get('search')))
            if url.path=='/api/documents':
                return self.send([{'name':p.stem,'path':p.relative_to(ds.root).as_posix()} for p in sorted(ds.root.glob('*.md'))])
            if url.path=='/api/live':return self.send(self.server.live.state())
            if url.path=='/api/network':return self.send(read_json(self.server.live.run/'network.json',{},max_size=32*1024*1024) if self.server.live.run else {})
            if url.path in ('/api/image','/api/download'):
                if url.path=='/api/image' and get('live')=='1':
                    if not self.server.live.run:raise FileNotFoundError('Noch kein Live-Bild.')
                    p=self.server.live.run/'dashboard-frame.ppm'
                else:p=safe_path(ds.root,get('path'))
                if not p.is_file():raise FileNotFoundError(p.name)
                if url.path=='/api/image':
                    if p.suffix.lower() not in ('.ppm','.png','.jpg','.jpeg','.bmp'):raise ValueError('Kein unterstütztes Bild.')
                    return self.send(ppm_png(p) if p.suffix=='.ppm' else p.read_bytes(),'image/png' if p.suffix=='.ppm' else (mimetypes.guess_type(p.name)[0] or 'application/octet-stream'))
                self.send_response(200);self.send_header('Content-Type','application/octet-stream');self.send_header('Content-Length',p.stat().st_size);self.send_header('Content-Disposition',"attachment; filename*=UTF-8''"+quote(p.name,safe=''));self.end_headers()
                with p.open('rb') as f:
                    while chunk:=f.read(256*1024):self.wfile.write(chunk)
                return
            target=safe_path(self.server.static,unquote(url.path).lstrip('/') or 'index.html')
            if not target.is_file():raise FileNotFoundError(target.name)
            self.send(target.read_bytes(),mimetypes.guess_type(target.name)[0] or 'application/octet-stream')
        except FileNotFoundError as e:self.send({'error':str(e)},status=404)
        except (ValueError,OSError) as e:self.send({'error':str(e)},status=400)
    def do_POST(self):
        try:
            self.trusted()
            if self.headers.get('X-Nerva-Token')!=self.server.token:return self.send({'error':'Ungültiger Sitzungsschlüssel.'},status=403)
            length=int(self.headers.get('Content-Length','0'))
            if not 0<=length<=16_384:raise ValueError('Anfrage zu groß.')
            body=json.loads(self.rfile.read(length) or '{}')
            if not isinstance(body,dict):raise ValueError('JSON-Objekt erwartet.')
            if self.path=='/api/live/start':return self.send(self.server.live.start(body))
            if self.path=='/api/live/control':return self.send(self.server.live.control(body.get('action')))
            self.send({'error':'Unbekannte Aktion.'},status=404)
        except (ValueError,OSError) as e:self.send({'error':str(e)},status=400)

def main():
    here=Path(__file__).resolve().parent
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root',type=Path,default=here.parent if here.name=='GUI' else here.parents[1]/'Messungen')
    parser.add_argument('--live',action='store_true');parser.add_argument('--port',type=int,default=None);parser.add_argument('--no-browser',action='store_true')
    args=parser.parse_args();root=args.root.resolve();static=here/'dist'
    if not root.is_dir():parser.error('Messverzeichnis fehlt.')
    if not (static/'index.html').exists():parser.error('GUI-Build fehlt. Siehe README des Dashboards.')
    script=here.parent/'Modell/run-demo.sh' if args.live else None
    if script and not script.is_file():parser.error('Modell-Startskript fehlt; Archiv ohne --live starten.')
    server=Server(('127.0.0.1',args.port or (8765 if args.live else 8766)),root,static,script)
    url=f'http://127.0.0.1:{server.server_port}'
    print(f'NERVA {"Live-Dashboard" if args.live else "Messarchiv"}: {url}\nBeenden mit Strg+C.',flush=True)
    if not args.no_browser:threading.Timer(.5,lambda:webbrowser.open(url)).start()
    try:server.serve_forever()
    except KeyboardInterrupt:pass
    finally:server.live.close();server.server_close()
if __name__=='__main__':main()
