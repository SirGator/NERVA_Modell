import React,{useMemo,useState} from 'react';
import {ZoomIn,ZoomOut,RotateCcw} from 'lucide-react';
import {Empty,Panel,DownloadButton} from './ui';
import {number,precise} from './api';
export default function Network({data={},live={}}){
 const [selected,setSelected]=useState(null),[zoom,setZoom]=useState(1),[all,setAll]=useState(false);
 const graph=useMemo(()=>{
  const nodes=data.neurons||[],core=nodes.filter(n=>n.role==='Core');
  const roles=new Map();for(const n of nodes){if(n.role==='Core')continue;if(!roles.has(n.role))roles.set(n.role,[]);roles.get(n.role).push(n)}
  const samples=Array.from(roles.values()).flatMap(group=>group.slice(0,128));
  const visible=(all?[...core,...samples]:core).slice(0,1024);
  const coords=visible.map(n=>({n,x:n.x+n.z*.45,y:n.y-n.z*.3}));
  const minX=Math.min(...coords.map(c=>c.x)),maxX=Math.max(...coords.map(c=>c.x));
  const minY=Math.min(...coords.map(c=>c.y)),maxY=Math.max(...coords.map(c=>c.y));
  const positioned=coords.map(({n,x,y})=>({...n,px:50+(x-minX)/(maxX-minX||1)*490,py:20+(y-minY)/(maxY-minY||1)*245}));
  const lookup=new Map(positioned.map(n=>[n.id,n]));
  const edges=(data.edges||[]).filter(e=>lookup.has(e.pre)&&lookup.has(e.post));
  return {nodes:positioned,lookup,edges:edges.slice(0,2500),totalEdges:edges.length,totalNodes:core.length};
 },[data,all]);
 const detail=data.neurons?.find(n=>n.id===selected);
 const active=new Set((live.raster||[]).filter(e=>e.time_s>live.simulation_us/1e6-.1).map(e=>e.neuron));
 const incident=detail?(data.edges||[]).filter(e=>e.pre===selected||e.post===selected).slice(0,8):[];
 return <Panel title="Neuronales Netz" subtitle={data.neurons?`${number(data.neurons.length)} Neuronen · ${number(data.edges?.length)} Synapsen · Snapshot Schritt ${data.step}`:'Räumliche Verschaltung und Membranwerte'} actions={<div className="tool-row"><button className="text-button" onClick={()=>setAll(v=>!v)}>{all?'Alle Populationen':'Core'}</button><button className="icon-button" aria-label="Netz vergrößern" onClick={()=>setZoom(z=>Math.min(3,z+.2))}><ZoomIn size={16}/></button><button className="icon-button" aria-label="Netz verkleinern" onClick={()=>setZoom(z=>Math.max(.6,z-.2))}><ZoomOut size={16}/></button><button className="icon-button" aria-label="Netzansicht zurücksetzen" onClick={()=>{setZoom(1);setSelected(null)}}><RotateCcw size={15}/></button></div>} className="network-panel">
  {graph.nodes.length?<div className="network-wrap"><svg viewBox="0 0 600 290" role="img" aria-label="Räumliche Projektion des gemessenen Netzwerks"><g transform={`translate(300 145) scale(${zoom}) translate(-300 -145)`}>
   {graph.edges.map(e=>{const a=graph.lookup.get(e.pre),b=graph.lookup.get(e.post),focus=e.pre===selected||e.post===selected;return <line key={e.id} x1={a.px} y1={a.py} x2={b.px} y2={b.py} stroke={focus?'#e8733d':'#bfc6cf'} strokeOpacity={focus?.65:.15} strokeWidth={focus?1.5:.5}/>})}
   {graph.nodes.map(n=><circle key={n.id} cx={n.px} cy={n.py} r={n.id===selected?5:active.has(n.id)?3.5:2.5} fill={n.id===selected||active.has(n.id)?'#e8733d':n.polarity==='Inhibitory'?'#677792':'#aeb7c3'} onClick={()=>setSelected(n.id)} className="neuron"><title>Neuron {n.id} · {n.role} · Potenzial {number(n.potential)} · {n.spikes} Spikes</title></circle>)}
  </g></svg><div className="network-caption"><span><i className="dot orange"/>aktiver Core <i className="dot blue"/>inhibitorisch</span><span>{graph.nodes.length} Zellen · {graph.edges.length}/{graph.totalEdges} Kanten sichtbar</span></div></div>:<div className="network-placeholder"><Empty>Wartet auf ersten Messwert</Empty></div>}
  {graph.nodes.length>0&&<div className="neuron-inspector"><label>Neuron-ID <input className="neuron-id" aria-label="Neuron-ID direkt auswählen" type="number" min="0" placeholder="ID" value={selected??''} onChange={e=>setSelected(e.target.value===''?null:Number(e.target.value))}/></label><label>Ansicht <select aria-label="Neuron auswählen" value={selected??''} onChange={e=>setSelected(e.target.value===''?null:Number(e.target.value))}><option value="">auswählen …</option>{graph.nodes.map(n=><option key={n.id} value={n.id}>{n.id} · {n.role}</option>)}</select></label>{detail?<><div className="neuron-values"><span>Potenzial <b>{precise(detail.potential)}</b></span><span>Schwelle <b>{precise(detail.threshold)}</b></span><span>Spikes <b>{number(detail.spikes)}</b></span><span>Wachstumsbedarf <b>{precise(detail.structural_drive)}</b></span></div><div className="edge-inspector">{incident.map(e=><span key={e.id}>{e.pre} → {e.post} <b>w {precise(e.weight)}</b></span>)}</div></>:<small>Zelle anklicken oder eine beliebige Neuron-ID eingeben. Snapshot alle 10 Schritte.</small>}</div>}
 {live.run&&data.neurons&&<div className="network-download"><DownloadButton path={`${live.run}/network.json`} label="Vollständigen Netzsnapshot herunterladen"/></div>}
 </Panel>
}
