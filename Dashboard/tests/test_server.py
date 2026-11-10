import importlib.util
import json
from pathlib import Path
import struct
import tempfile
import threading
import unittest
from urllib.error import HTTPError
from urllib.request import Request,urlopen
import zlib

spec=importlib.util.spec_from_file_location('dashboard_server',Path(__file__).resolve().parents[1]/'server.py')
server=importlib.util.module_from_spec(spec);spec.loader.exec_module(server)

class DashboardTests(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory();self.root=Path(self.temp.name).resolve();self.run=self.root/'Experimente/m8-confirm';self.run.mkdir(parents=True)
        (self.run/'manifest.json').write_text(json.dumps({'seeds':[510,511],'conditions':[{'id':'reference'}],'fixed':'historical protocol description'}))
        (self.run/'summary.csv').write_text('case,seeds,checkpoint_probe,checkpoint_probe_confirmed\nreference,2,0.5,False\n')
        (self.run/'rows.csv').write_text('step,arm,value\n'+''.join(f'{i},{"paired" if i%2 else "before"},{i*2}\n' for i in range(240)))
        (self.root/'README.md').write_text('# Forschung\n')
        self.ds=server.Dataset(self.root)
    def tearDown(self):self.temp.cleanup()
    def test_metadata_and_actual_summary_values(self):
        runs=self.ds.runs();self.assertEqual(runs[0]['seeds'],[510,511]);self.assertEqual(runs[0]['conditions'],1)
        detail=self.ds.detail('Experimente/m8-confirm');self.assertEqual(detail['tables'][0]['rows'][0]['checkpoint_probe'],'0.5')
        self.assertEqual(detail['tables'][0]['rows'][0]['checkpoint_probe_confirmed'],'False')
    def test_documents_include_organized_folders_without_duplicate_legacy_links(self):
        for bucket in ('Dokumentation','Berichte','Darstellung'):
            folder=self.root/bucket;folder.mkdir();(folder/'guide.md').write_text('# Guide\n')
        (self.root/'legacy.md').symlink_to('Berichte/guide.md')
        paths={d['path'] for d in self.ds.documents()}
        self.assertEqual(paths,{'README.md','Dokumentation/guide.md','Berichte/guide.md','Darstellung/guide.md'})
        preview=self.ds.preview('legacy.md')
        self.assertEqual(preview['path'],'Berichte/guide.md')
        self.assertEqual(preview['kind'],'markdown')
    def moved_document_dataset(self):
        measurements=self.root/'project/Messungen';measurements.mkdir(parents=True)
        docs=measurements.parent/'Doku';reports=docs/'Berichte';reports.mkdir(parents=True)
        (reports/'M6_EXPERIMENT.md').write_text('# Moved report\n')
        (docs/'README.md').write_text('# Documentation\n')
        return measurements,docs,server.Dataset(measurements)
    def test_moved_documents_and_legacy_entrypoints_use_a_bounded_markdown_root(self):
        measurements,docs,dataset=self.moved_document_dataset()
        paths={d['path'] for d in dataset.documents()}
        self.assertEqual(paths,{'Doku/README.md','Doku/Berichte/M6_EXPERIMENT.md'})
        preview=dataset.preview('M6_EXPERIMENT.md')
        self.assertEqual(preview['path'],'Doku/Berichte/M6_EXPERIMENT.md')
        self.assertEqual(preview['text'],'# Moved report\n')
        (docs/'escape.md').symlink_to('/etc/passwd')
        for value in ('Doku/../Messungen/README.md','Doku/.git/config.md','Doku/escape.md','Doku/source.rs'):
            with self.assertRaises(ValueError):dataset.preview(value)
        # Run data never gains access to the sibling directory.
        with self.assertRaises(ValueError):server.safe_path(measurements,'../Doku/README.md')
    def test_moved_document_can_be_previewed_and_downloaded_through_the_api(self):
        measurements,docs,dataset=self.moved_document_dataset()
        http=server.Server(('127.0.0.1',0),measurements,self.root)
        thread=threading.Thread(target=http.serve_forever,daemon=True);thread.start()
        base=f'http://127.0.0.1:{http.server_port}'
        try:
            with urlopen(base+'/api/documents') as response:
                self.assertIn('Doku/README.md',{d['path'] for d in json.load(response)})
            with urlopen(base+'/api/download?path=Doku/Berichte/M6_EXPERIMENT.md') as response:
                self.assertEqual(response.read(),b'# Moved report\n')
            with urlopen(base+'/api/preview?path=Doku/Berichte/M6_EXPERIMENT.md') as response:
                self.assertEqual(json.load(response)['kind'],'markdown')
            with self.assertRaises(HTTPError) as error:
                urlopen(base+'/api/download?path=Doku/../Messungen/README.md')
            self.assertEqual(error.exception.code,400)
        finally:
            http.shutdown();http.server_close();thread.join()
    def test_catalog_distinguishes_unexecuted_plans_from_confirmation_runs(self):
        folder=self.root/'Dokumentation';folder.mkdir()
        (folder/'MESSREIHEN.json').write_text(json.dumps({'runs':[{'id':'Experimente/m8-confirm','kind':'Planungsstand'}]}))
        self.assertEqual(self.ds.runs()[0]['kind'],'Planungsstand')
    def test_paged_csv_and_filtered_continuation_do_not_drop_rows(self):
        p=self.run/'rows.csv';first=server.csv_preview(p);second=server.csv_preview(p,first['next_offset']);third=server.csv_preview(p,second['next_offset'])
        all_rows=first['rows']+second['rows']+third['rows'];self.assertEqual([int(r['step']) for r in all_rows],list(range(240)))
        first=server.csv_preview(p,search='paired');second=server.csv_preview(p,first['next_offset'],search='paired')
        self.assertEqual([int(r['step']) for r in first['rows']+second['rows']],list(range(1,240,2)))
    def test_path_traversal_hidden_files_and_symlinks_are_rejected(self):
        for value in ('../../etc/passwd','.git/config','Experimente/../.git/config'):
            with self.assertRaises(ValueError):server.safe_path(self.root,value)
        (self.root/'outside').symlink_to('/etc')
        with self.assertRaises(ValueError):server.safe_path(self.root,'outside/passwd')
    def test_ppm_whitespace_pixels_are_preserved(self):
        p=self.root/'test.ppm';pixels=b' \n\r\x00\x7f\xff';p.write_bytes(b'P6\n# capture\n2 1\n255\n'+pixels)
        png=server.ppm_png(p);self.assertEqual(png[:8],b'\x89PNG\r\n\x1a\n')
        cursor=8;compressed=b''
        while cursor<len(png):
            n=struct.unpack('>I',png[cursor:cursor+4])[0];kind=png[cursor+4:cursor+8]
            if kind==b'IDAT':compressed+=png[cursor+8:cursor+8+n]
            cursor+=12+n
        self.assertEqual(zlib.decompress(compressed),b'\0'+pixels)
    def test_http_origin_token_and_archive_read_only(self):
        static=self.root/'static';static.mkdir();(static/'index.html').write_text('<h1>NERVA</h1>')
        http=server.Server(('127.0.0.1',0),self.root,static)
        thread=threading.Thread(target=http.serve_forever,daemon=True);thread.start();base=f'http://127.0.0.1:{http.server_port}'
        try:
            with urlopen(base+'/api/config') as response:config=json.load(response)
            self.assertFalse(config['live'])
            for headers in ({'Content-Type':'application/json'},{'Content-Type':'application/json','X-Nerva-Token':config['token'],'Origin':'http://attacker.invalid'}):
                with self.assertRaises(HTTPError) as error:urlopen(Request(base+'/api/live/start',data=b'{}',headers=headers))
                self.assertIn(error.exception.code,(400,403));error.exception.close()
            with self.assertRaises(HTTPError) as error:urlopen(Request(base+'/api/live/start',data=b'{}',headers={'X-Nerva-Token':config['token']}))
            self.assertEqual(error.exception.code,400);error.exception.close()
            with urlopen(base+'/api/runs') as response:self.assertEqual(len(json.load(response)),1)
        finally:http.shutdown();http.server_close();thread.join()
    def test_lfs_pointer_is_not_displayed_as_data(self):
        for suffix in ('.csv','.json'):
            p=self.run/('unfetched'+suffix);p.write_text('version https://git-lfs.github.com/spec/v1\noid sha256:abc\nsize 999\n')
            result=self.ds.preview('Experimente/m8-confirm/'+p.name)
            self.assertEqual(result['kind'],'text');self.assertIn('git lfs pull',result['text'])

if __name__=='__main__':unittest.main()
