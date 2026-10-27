import React,{useEffect,useRef,useState} from 'react';
import {Play,Pause,Square,Snowflake,Clock3,Activity,GitBranch,Database,Info} from 'lucide-react';
import {api,number} from './api';
import {Panel,Metric,Empty,ErrorBox} from './ui';
import {LineChart,Raster,Legend} from './charts';
import Network from './Network';
const STATUS={ready:'Bereit',starting:'Wird vorbereitet',running:'Läuft',paused:'Pausiert',completed:'Abgeschlossen',stopped:'Gestoppt',error:'Fehler'};
const METRICS=[{key:'core',label:'Core',color:'#e8733d'},{key:'sensory',label:'Sensorik',color:'#508abd'},{key:'motor',label:'Motorik',color:'#5f9473'}];
export default function Live({config,onRunsChanged}){
 const [state,setState]=useState({status:'ready'}),[network,setNetwork]=useState({}),[error,setError]=useState(''),[busy,setBusy]=useState(false),[hidden,setHidden]=useState(new Set()),[imageError,setImageError]=useState(false);
 const [settings,setSettings]=useState({seed:42,processing:256,steps:1000,keyboard:'full',learning:true,development:true,homeostasis:true,feedback:true});
 const lastRun=useRef(''),lastNetwork=useRef(-1);
 useEffect(()=>{setImageError(false)},[state.step]);
 useEffect(()=>{
  let alive=true,timer;const controller=new AbortController();
  async function poll(){try{
   const value=await api('/api/live',{signal:controller.signal});if(!alive)return;
   if(lastRun.current!==value.run){lastRun.current=value.run;lastNetwork.current=-1;setNetwork({});setImageError(false);}
   setState(value);
   if(value.step&&Math.floor(value.step/10)!==lastNetwork.current){const n=await api('/api/network',{signal:controller.signal});if(alive&&n.neurons){setNetwork(n);lastNetwork.current=Math.floor(value.step/10);}}
  }catch(e){if(alive&&e.name!=='AbortError')setError(e.message);}finally{if(alive)timer=setTimeout(poll,800);}}
  poll();return()=>{alive=false;controller.abort();clearTimeout(timer);};
 },[]);
 async function command(action){setBusy(true);setError('');try{
  const value=await api(action==='start'?'/api/live/start':'/api/live/control',{method:'POST',headers:{'Content-Type':'application/json','X-Nerva-Token':config.token},body:JSON.stringify(action==='start'?settings:{action})});
  setState(value);if(action==='start')onRunsChanged?.();
 }catch(e){setError(e.message);}finally{setBusy(false);}}
 const running=state.managed,paused=state.status==='paused'||state.paused,totals=state.totals||{};
 const set=(key,value)=>setSettings(s=>({...s,[key]:value}));
 const series=METRICS.map(m=>({...m,hidden:hidden.has(m.key)}));
 return <>
  <header className="page-heading"><div><h1>Live-Dashboard</h1><p>Wahrnehmung, Aktivität und lokale Plastizität.</p></div><div className="live-actions"><span className={`status ${state.status}`}><i/>{STATUS[state.status]||state.status}</span>{running?<><button className="button secondary" disabled={busy||state.status==='starting'} onClick={()=>command(paused?'resume':'pause')}>{paused?<Play size={16}/>:<Pause size={16}/>}<span>{paused?'Fortsetzen':'Pause'}</span></button><button className="button secondary stop" disabled={busy} onClick={()=>command('stop')}><Square size={14}/>Stop</button></>:<button className="button primary" disabled={busy} onClick={()=>command('start')}><Play size={16}/>{busy?'Startet …':'Lauf starten'}</button>}</div></header>
  <ErrorBox message={error||state.error}/>
  <section className="run-config" aria-label="Lauf konfigurieren">
   <label>Seed<input aria-label="Seed" type="number" min="0" max="18446744073709551615" disabled={running} value={settings.seed} onChange={e=>set('seed',e.target.value)}/></label>
   <label>Processing<input aria-label="Processing" type="number" min="1" max="4096" disabled={running} value={settings.processing} onChange={e=>set('processing',Number(e.target.value))}/></label>
   <label>Schritte<input aria-label="Schritte" type="number" min="1" max="10000" disabled={running} value={settings.steps} onChange={e=>set('steps',Number(e.target.value))}/></label>
   <label>Tastatur<select disabled={running} value={settings.keyboard} onChange={e=>set('keyboard',e.target.value)}><option value="full">Vollständig</option><option value="wasd">WASD</option><option value="ad">A / D</option></select></label>
  </section>
  <div className="config-flags">{[['learning','Lernen'],['development','Strukturentwicklung'],['homeostasis','Homöostase'],['feedback','Aktionsrückführung']].map(([key,label])=><label key={key}><input type="checkbox" checked={running&&typeof state[key]==='boolean'?state[key]:settings[key]} disabled={running} onChange={e=>set(key,e.target.checked)}/>{label}</label>)}{running&&state.learning&&<button className="text-button" disabled={busy} onClick={()=>command('freeze')}><Snowflake size={14}/>Lernen einfrieren</button>}{state.learning===false&&state.step>0&&<span className="muted">Lernregeln eingefroren</span>}</div>
  <div className="metrics four"><Metric label="Simulationszeit" value={state.simulation_us?`${number(state.simulation_us/1e6)} s`:'—'} detail={state.step?`Schritt ${number(state.step)} / ${number(state.requested_steps)}`:'50 ms pro Simulationsschritt'} icon={Clock3}/><Metric label="Core-Spikes" value={number(totals.core)} detail={totals.sensory!=null?`${number(totals.sensory)} sensorische Spikes`:'Noch keine Messwerte'} icon={Activity}/><Metric label="Gewichtsänderungen" value={number(totals.weights)} detail={totals.homeostasis!=null?`${number(totals.homeostasis)} Homöostase-Updates`:'Lokale synaptische Plastizität'} icon={Database}/><Metric label="Core-Synapsen" value={number(state.core_synapses)} detail={totals.formed!=null?`+${number(totals.formed)} gebildet · −${number(totals.pruned)} entfernt`:'Wachstum und Pruning'} icon={GitBranch}/></div>
  {running&&<div className="progress" aria-label="Fortschritt"><div style={{width:`${100*(state.step||0)/(state.requested_steps||settings.steps)}%`}}/></div>}
  <div className="live-main"><Network data={network} live={state}/><Panel title="Wahrnehmung" subtitle="Eingangsbild des zuletzt gemessenen Schritts" className="perception-panel"><div className="frame">{state.step&&!imageError?<img alt="Aktuelles Eingangsbild der virtuellen Maschine" src={`/api/image?live=1&step=${state.step}`} onError={()=>setImageError(true)} onLoad={()=>setImageError(false)}/>:<Empty>{state.status==='starting'?'VM startet und Modell wird vorbereitet …':'Noch kein VM-Bild'}</Empty>}</div><div className="keyboard-title"><h3>Ausgeführte Tasten</h3><span>{state.motors?.length||0} Kanäle</span></div><div className="keyboard">{state.motors?.length?state.motors.map(k=><div key={k.qcode} className={(state.keys||[]).includes(k.label)?'pressed':''} title={`${k.label}: ${k.spikes} Spikes`}><span>{k.label}</span><small>{k.spikes||'·'}</small></div>):['Q','W','E','R','T','Z','A','S','D','F','G','H','Y','X','C','V'].map(k=><div key={k} className="idle-key">{k}</div>)}</div><div className="feedback-line">{state.keys?.length?`Ausgeführt: ${state.keys.join(' + ')}`:'Keine Taste ausgeführt'}<span>{number(totals.motor)} Motor-Spikes</span></div></Panel></div>
  <Panel title="Aktivität im Zeitverlauf" subtitle="Spikes pro 50-ms-Fenster · letzte 300 Schritte" actions={<Legend series={series} onToggle={key=>setHidden(h=>{const next=new Set(h);next.has(key)?next.delete(key):next.add(key);return next;})}/>}><LineChart rows={state.history||[]} series={series.filter(s=>!s.hidden)} height={190}/></Panel>
  <div className="live-bottom"><Panel title="Spike-Raster" subtitle="Core-Neuronen · letzte 4.000 Ereignisse, maximal 5 s"><Raster events={state.raster||[]} ids={state.core_ids||[]}/></Panel><Panel title="Laufprotokoll" subtitle={state.run||'Ausgaben des Modellprozesses'}><div className="log">{state.logs?.length?<pre>{state.logs.slice(-35).join('\n')}</pre>:<Empty>Noch keine Einträge</Empty>}</div></Panel></div>
  <div className="bottom-note"><Info size={14}/>Lokales Dashboard · Simulationszeit und reale Laufzeit sind unabhängig.</div>
 </>
}
