"use strict";
const fs=require("fs"), path=require("path"), zlib=require("zlib");
const ROOT=process.env.DIST || ".upstream/zigbee2mqtt/node_modules/zigbee-herdsman-converters/dist";
const files=fs.readdirSync(path.join(ROOT,"devices")).filter(f=>f.endsWith(".js")&&f!=="index.js");
const candidates={}; let definitions=0, fpCount=0, wlCount=0, max=0, maxKey="";
for (const file of files) {
  const mod=require(path.join(ROOT,"devices",file));
  const defs=mod.definitions||mod.default||mod;
  defs.forEach((d,index)=>{
    definitions++;
    const base={f:file,i:index,m:d.model,v:d.vendor};
    const zbs=d.zigbeeModel?(Array.isArray(d.zigbeeModel)?d.zigbeeModel:[d.zigbeeModel]):[];
    for(const zb of zbs){ const k=zb?zb.toLowerCase():"null"; (candidates[k]||(candidates[k]=[])).push({...base,zb:zb||null}); }
    for(const fp of (d.fingerprint||[])){ fpCount++; const k=fp.modelID?fp.modelID.toLowerCase():"null"; (candidates[k]||(candidates[k]=[])).push({...base,fp:{modelID:fp.modelID,manufacturerName:fp.manufacturerName,manufacturerID:fp.manufacturerID,type:fp.type,priority:fp.priority,powerSource:fp.powerSource,applicationVersion:fp.applicationVersion,stackVersion:fp.stackVersion,hardwareVersion:fp.hardwareVersion,dateCode:fp.dateCode,softwareBuildID:fp.softwareBuildID}}); }
    for(const wl of (d.whiteLabel||[])){ wlCount++; if(wl.fingerprint) for(const fp of wl.fingerprint){ fpCount++; const k=fp.modelID?fp.modelID.toLowerCase():"null"; (candidates[k]||(candidates[k]=[])).push({...base,m:wl.model,v:wl.vendor||d.vendor,fp:{modelID:fp.modelID,manufacturerName:fp.manufacturerName,manufacturerID:fp.manufacturerID,type:fp.type,priority:fp.priority,powerSource:fp.powerSource,applicationVersion:fp.applicationVersion,stackVersion:fp.stackVersion,hardwareVersion:fp.hardwareVersion,dateCode:fp.dateCode,softwareBuildID:fp.softwareBuildID}}); } }
    if (zbs.length===0 && !d.fingerprint?.length) { (candidates["null"]||(candidates["null"]=[])).push({...base,zb:null}); }
  });
}
for(const [k,v] of Object.entries(candidates)){ if(v.length>max){max=v.length;maxKey=k;} }
const json=JSON.stringify(candidates);
const simple=JSON.stringify(Object.fromEntries(Object.entries(candidates).map(([k,v])=>[k,v.map(x=>[x.f,x.i])])));
const compact=JSON.stringify({v:1,c:Object.fromEntries(Object.entries(candidates).map(([k,v])=>[k,v.map(x=>[x.f,x.i,x.m,x.v,x.zb,x.fp||null])]))});
const gz=s=>zlib.gzipSync(s,{level:9});
console.log(JSON.stringify({definitions,files:files.length,keys:Object.keys(candidates).length,fpCount,wlCount,maxCandidates:max,maxKey,bytes:{originalModelIndex:fs.statSync(path.join(ROOT,"models-index.json")).size,simple:simple.length,richJSON:json.length,compact:compact.length,richGzip:gz(json).length,compactGzip:gz(compact).length}},null,2));
fs.writeFileSync("/tmp/opencode/rich-index.json",json);
fs.writeFileSync("/tmp/opencode/compact-index.json",compact);
