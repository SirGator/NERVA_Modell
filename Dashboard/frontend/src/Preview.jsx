import React,{useEffect,useState} from 'react';
import {X,Search} from 'lucide-react';
import {api,query,num} from './api';
import {Panel,DataTable,Empty,ErrorBox,DownloadButton,Paging} from './ui';
import {LineChart} from './charts';
export function Markdown({text='',onNavigate}){
 // Render document content as React text nodes: archived HTML is never executed.
 const inline=(line)=>line.split(/(\[[^\]]+\]\([^)]+\)|`[^`]+`|\*\*[^*]+\*\*)/g).map((part,i)=>{
  const link=part.match(/^\[([^\]]+)\]\(([^)]+)\)$/);
  if(link){const external=/^https?:\/\//.test(link[2]);if(!external&&/^[a-z]+:/i.test(link[2]))return <span key={i}>{link[1]}</span>;return <a key={i} href={external?link[2]:'#'} target={external?'_blank':undefined} rel={external?'noreferrer':undefined} onClick={external?undefined:e=>{e.preventDefault();onNavigate?.(link[2]);}}>{link[1]}</a>}
  if(part.startsWith('`'))return <code key={i}>{part.slice(1,-1)}</code>;
  if(part.startsWith('**'))return <strong key={i}>{part.slice(2,-2)}</strong>;
  return part;
 });
 let fenced=false,buffer=[],nodes=[];
 for(const [i,line]of text.split('\n').entries()){
  if(line.startsWith('```')){if(fenced){nodes.push(<pre key={i}>{buffer.join('\n')}</pre>);buffer=[];}fenced=!fenced;continue;}
  if(fenced){buffer.push(line);continue;}
  const heading=line.match(/^(#{1,6})\s+(.*)$/);if(heading){const Tag=`h${Math.min(4,heading[1].length)}`;nodes.push(<Tag key={i}>{inline(heading[2])}</Tag>);}
  else if(line.startsWith('|'))nodes.push(<div key={i} className="markdown-table-row">{inline(line)}</div>);
  else if(line.startsWith('>'))nodes.push(<blockquote key={i}>{inline(line.slice(1).trim())}</blockquote>);
  else if(/^[-*] /.test(line))nodes.push(<div key={i} className="markdown-list">• {inline(line.slice(2))}</div>);
  else if(line.trim())nodes.push(<p key={i}>{inline(line)}</p>);
 }
 if(buffer.length)nodes.push(<pre key="last">{buffer.join('\n')}</pre>);
 return <article className="markdown">{nodes}</article>;
}
export default function Preview({path,onClose,onNavigate,embedded=false}){
 const [data,setData]=useState(null),[error,setError]=useState(''),[offset,setOffset]=useState(0),[search,setSearch]=useState(''),[searchInput,setSearchInput]=useState(''),[axes,setAxes]=useState({x:'',y:''});
 useEffect(()=>{setOffset(0);setSearch('');setSearchInput('');setAxes({x:'',y:''});},[path]);
 useEffect(()=>{const controller=new AbortController();setData(null);setError('');api(query('/api/preview',{path,offset,search}),{signal:controller.signal}).then(setData).catch(e=>{if(e.name!=='AbortError')setError(e.message)});return()=>controller.abort();},[path,offset,search]);
 const numericColumns=data?.columns?.filter(c=>data.rows?.some(r=>num(r[c])!==null))||[];
 const body=<><ErrorBox message={error}/>{!data&&!error?<Empty loading>Datei wird gelesen …</Empty>:null}{data?.kind==='csv'?<><div className="preview-tools"><form onSubmit={e=>{e.preventDefault();setSearch(searchInput);setOffset(0)}}><Search size={15}/><input aria-label="CSV-Zeilen filtern" placeholder="Zeilen durchsuchen …" value={searchInput} onChange={e=>setSearchInput(e.target.value)}/><button className="text-button">Filtern</button></form><label>Diagramm<select aria-label="Y-Spalte für Diagramm" value={axes.y} onChange={e=>setAxes(a=>({...a,y:e.target.value}))}><option value="">Messgröße wählen …</option>{numericColumns.map(c=><option key={c}>{c}</option>)}</select></label>{axes.y&&<label>X-Achse<select aria-label="X-Spalte für Diagramm" value={axes.x} onChange={e=>setAxes(a=>({...a,x:e.target.value}))}><option value="">Zeilenindex</option>{data.columns.map(c=><option key={c}>{c}</option>)}</select></label>}</div>{axes.y&&<div className="file-chart"><h3>{axes.y} · aktuelle Vorschauseite</h3><LineChart rows={data.rows.map((r,i)=>({...r,_index:offset+i+1}))} x={axes.x||'_index'} series={[{key:axes.y,label:axes.y}]} height={220}/></div>}<DataTable columns={data.columns} rows={data.rows} compact/><Paging offset={offset} nextOffset={data.next_offset} hasMore={data.has_more} rows={data.rows.length} onChange={setOffset}/><div className="preview-note">Vorschau: maximal 100 Zeilen pro Seite. {data.limited?'Suchlauf auf 50.000 Zeilen begrenzt; mit der nächsten Seite fortsetzen.':'Große Dateien werden nicht vollständig in den Browser geladen.'}</div></>:null}
 {data?.kind==='image'&&<div className="image-preview"><img alt={path.split('/').pop()} src={query('/api/image',{path})}/></div>}
 {data?.kind==='markdown'&&<Markdown text={data.text} onNavigate={onNavigate}/>}
 {data?.kind==='text'&&<pre className="text-preview">{data.text}</pre>}
 {data?.truncated&&<p className="preview-note">Vorschau auf 256 kB begrenzt. Die Originaldatei steht vollständig zum Download bereit.</p>}
 </>;
 if(embedded)return body;
 return <section className="file-preview" aria-label="Dateivorschau"><div className="preview-title"><div><span>Dateivorschau</span><h2>{path.split('/').pop()}</h2><small>{path}</small></div><div className="tool-row"><DownloadButton path={path}/><button className="icon-button" aria-label="Dateivorschau schließen" onClick={onClose}><X size={20}/></button></div></div>{body}</section>
}
