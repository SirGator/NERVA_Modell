import React,{useState} from 'react';
import {AlertCircle,Inbox,LoaderCircle,Download,ArrowLeft,ArrowRight} from 'lucide-react';
import {number,num,query} from './api';
export function Panel({title,subtitle,actions,children,className=''}){return <section className={`panel ${className}`}><div className="panel-heading"><div><h2>{title}</h2>{subtitle&&<p>{subtitle}</p>}</div>{actions}</div>{children}</section>}
export function Empty({children='Noch keine Daten',loading=false}){const Icon=loading?LoaderCircle:Inbox;return <div className="empty"><Icon size={24} className={loading?'spin':''}/><span>{children}</span></div>}
export function ErrorBox({message}){return message?<div className="error" role="alert"><AlertCircle size={17}/><span>{message}</span></div>:null}
export function Metric({label,value,detail,icon:Icon}){return <div className="metric"><div className="metric-label">{label}{Icon&&<Icon size={19}/>}</div><strong>{value}</strong>{detail&&<small>{detail}</small>}</div>}
const probabilities=/^(visual|paired|before|aged|scrambled|probe|checkpoint_probe|rest_checkpoint_probe|retained_probe|interference_probe|early16|balanced_accuracy)$/;
export function display(value,column=''){
 if(value==null||value==='')return '—';
 if(value===false||value==='False')return <span className="criteria"><i/>Nicht erfüllt</span>;
 if(value===true||value==='True')return <span className="criteria yes"><i/>Erfüllt</span>;
 if(probabilities.test(column)&&num(value)!==null)return `${number(Number(value)*100)} %`;
 // Preserve original CSV precision, especially small STDP weight changes.
 if(num(value)!==null)return String(value);
 return String(value);
}
export const LABELS={case:'Bedingung',case_id:'Bedingung',seeds:'Seeds',n_seeds:'Seeds',checkpoint_probe:'Sofort',rest_checkpoint_probe:'Nach Ruhe',probe:'Online',visual:'Visuell',paired:'Gekoppelt',before:'Vor Training',aged:'Eingefroren',scrambled:'Entkoppelt',checkpoint_probe_confirmed:'Sofort: Kriterien',rest_checkpoint_probe_confirmed:'Ruhe: Kriterien'};
export function DataTable({columns=[],rows=[],compact=false}){return <div className="table-scroll" tabIndex={0}><table className={compact?'compact':''}><thead><tr>{columns.map(c=><th key={c} title={c}>{LABELS[c]||c}</th>)}</tr></thead><tbody>{rows.map((r,i)=><tr key={i}>{columns.map(c=><td key={c} title={typeof r[c]==='string'?r[c]:undefined}>{display(r[c],c)}</td>)}</tr>)}</tbody></table>{!rows.length&&<Empty>Keine passenden Zeilen</Empty>}</div>}
export function Paging({offset,nextOffset,hasMore,onChange,rows}){return <div className="pagination"><span>{number(rows)} Zeilen in dieser Vorschau · ab Zeile {number(offset+1)}</span><div><button className="icon-button" aria-label="Zurück zur ersten Seite" disabled={offset===0} onClick={()=>onChange(0)}><ArrowLeft size={17}/></button><button className="icon-button" aria-label="Nächste Zeilen" disabled={!hasMore} onClick={()=>onChange(nextOffset)}><ArrowRight size={17}/></button></div></div>}
export function DownloadButton({path,label='Original herunterladen'}){return <a className="button secondary" href={query('/api/download',{path})}><Download size={15}/>{label}</a>}
