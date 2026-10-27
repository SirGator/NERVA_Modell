export async function api(path, options={}) {
  const response=await fetch(path,options);
  const data=await response.json();
  if(!response.ok) throw new Error(data.error||`Anfrage fehlgeschlagen (${response.status})`);
  return data;
}
export const query=(path,values)=>`${path}?${new URLSearchParams(values)}`;
export function num(value){if(value===null||typeof value==='boolean'||(typeof value==='string'&&!value.trim()))return null;const n=Number(value);return Number.isFinite(n)?n:null;}
export const number=value=>value===undefined||value===null?'—':new Intl.NumberFormat('de-DE',{maximumFractionDigits:2}).format(value);
export const size=value=>value==null?'Ordner':value>1e6?`${number(value/1e6)} MB`:value>1e3?`${number(value/1e3)} kB`:`${value} B`;
export const formatDate=value=>new Date(value*1000).toLocaleDateString('de-DE',{day:'2-digit',month:'short',year:'numeric'});

export const precise=value=>value==null?'—':new Intl.NumberFormat('de-DE',{maximumSignificantDigits:8}).format(value);
