import React,{useEffect,useMemo,useRef,useState} from 'react';
import {num,number} from './api';
import {Empty} from './ui';
function useChartWidth(active){
 const ref=useRef(null),[width,setWidth]=useState(900);
 useEffect(()=>{if(!ref.current)return;const observer=new ResizeObserver(entries=>setWidth(entries[0].contentRect.width));observer.observe(ref.current);return()=>observer.disconnect()},[active]);
 return [ref,width];
}
const COLORS=['#e8733d','#508abd','#5f9473'];
export function LineChart({rows=[],series=[],x='step',percent=false,height=190,chance=false,category=false}){
 const [chartRef,available]=useChartWidth(rows.length>0);
 const prepared=useMemo(()=>{
  const usable=series.filter(s=>rows.some(r=>num(r[s.key])!==null));
  const values=rows.flatMap(r=>usable.map(s=>num(r[s.key]))).filter(v=>v!==null);
  const focused=percent&&chance&&values.length&&Math.min(...values)>=.3&&Math.max(...values)<=.7;
  const max=percent?(focused?.7:1):Math.max(1,...values);const min=percent?(focused?.3:0):Math.min(0,...values);
  const left=52,right=20,top=18,bottom=category?76:32,width=Math.max(category?650:320,available),plotH=height-top-bottom;
  const X=i=>left+i*(width-left-right)/Math.max(1,rows.length-1);
  const Y=v=>top+(max-v)/(max-min||1)*plotH;
  return {usable,min,max,left,right,top,bottom,width,plotH,X,Y};
 },[rows,series,percent,height,category,chance,available]);
 if(!rows.length)return <div className="chart-empty" style={{height}}><Empty>Noch keine Messwerte</Empty></div>;
 const {usable,min,max,left,right,top,width,plotH,X,Y}=prepared;
 const ticks=category?rows.map((r,i)=>i):Array.from(new Set([0,Math.floor((rows.length-1)/2),rows.length-1]));
 return <div className="chart-viewport" ref={chartRef}><svg className="chart" data-category={category} viewBox={`0 0 ${width} ${height}`} style={{width:category?width:'100%',height}} role="img" aria-label="Diagramm mit gemessenen Werten">
  {[0,1,2,3,4].map(i=>{const value=min+(max-min)*i/4;const y=Y(value);return <g key={i}><line x1={left} x2={width-right} y1={y} y2={y} stroke="#edf0f3"/><text x={left-10} y={y+4} textAnchor="end">{number(value*(percent?100:1))}{percent?' %':''}</text></g>})}
  {chance&&<g><line x1={left} x2={width-right} y1={Y(.5)} y2={Y(.5)} stroke="#aeb6c2" strokeDasharray="5 4"/><text x={width-right} y={Y(.5)-8} textAnchor="end">Zufallsniveau 50 %</text></g>}
  {usable.map((s,j)=><g key={s.key}>
   {!category&&<path d={rows.map((r,i)=>{const v=num(r[s.key]);return v===null?'':`${i===0||num(rows[i-1]?.[s.key])===null?'M':'L'}${X(i)},${Y(v)}`}).join(' ')} fill="none" stroke={s.color||COLORS[j%3]} strokeWidth="2" strokeLinejoin="round"/>}
   {(category||rows.length<50)&&rows.map((r,i)=>num(r[s.key])!==null?<circle key={i} cx={X(i)} cy={Y(Number(r[s.key]))} r={category?4:2.5} fill={s.color||COLORS[j%3]}><title>{r[x]} · {s.label||s.key}: {number(Number(r[s.key])*(percent?100:1))}{percent?' %':''}</title></circle>:null)}
  </g>)}
  {ticks.map(i=><text key={i} x={X(i)} y={top+plotH+18} textAnchor={category?'end':'middle'} transform={category?`rotate(-45 ${X(i)} ${top+plotH+18})`:undefined}>{String(rows[i]?.[x]??i).slice(0,25)}</text>)}
 </svg></div>
}
export function Legend({series,onToggle}){return <div className="legend">{series.map((s,i)=><button key={s.key} className={s.hidden?'off':''} onClick={()=>onToggle?.(s.key)} disabled={!onToggle}><i style={{background:s.color||COLORS[i%3]}}/>{s.label||s.key}</button>)}</div>}
export function Raster({events=[],ids=[]}){
 const [rasterRef,available]=useChartWidth(events.length>0);
 if(!events.length)return <div className="raster-empty"><Empty>Noch keine Core-Spikes</Empty></div>;
 const width=Math.max(320,available),height=160,left=50,top=12;
 const high=events[events.length-1].time_s,low=Math.max(0,high-5);
 const lookup=new Map(ids.map((id,i)=>[id,i]));
 return <div className="chart-viewport" ref={rasterRef}><svg className="chart" viewBox={`0 0 ${width} ${height}`} style={{height:160}} role="img" aria-label="Gemessene Core-Spikezeiten">
  {[0,.5,1].map(i=><g key={i}><line x1={left} x2={width-15} y1={top+i*115} y2={top+i*115} stroke="#edf0f3"/><text x={left-9} y={top+i*115+4} textAnchor="end">{ids[Math.min(ids.length-1,Math.round(i*(ids.length-1)))]}</text></g>)}
  {events.filter(e=>e.time_s>=low).map((e,i)=>{const y=top+(lookup.get(e.neuron)||0)/Math.max(1,ids.length-1)*115;return <line key={i} x1={left+(e.time_s-low)/(high-low||1)*(width-left-15)} x2={left+(e.time_s-low)/(high-low||1)*(width-left-15)} y1={y-2} y2={y+2} stroke="#e8733d" opacity=".6"/>})}
  <text x={left} y="152">{number(low)} s</text><text x={width-15} y="152" textAnchor="end">{number(high)} s Simulationszeit</text>
 </svg></div>
}
