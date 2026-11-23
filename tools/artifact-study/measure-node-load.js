"use strict";
const fs = require("fs");
const path = require("path");
const ROOT = process.env.ROOT || "/opt/zigbee2mqtt/node_modules/zigbee-herdsman-converters";
const mode = process.argv[2] || "root";
function kb(n){return Math.round(n/1024*10)/10;}
function sizeOf(p){try{return fs.statSync(p).size}catch(e){return 0}}
function loadedStats(){
  let bytes=0, files=0;
  for (const p of Object.keys(require.cache)) {
    if (p.startsWith(ROOT)) { bytes += sizeOf(p); files++; }
  }
  return {files, bytes, kb: kb(bytes)};
}
function rss(){return {rssKb: Math.round(process.memoryUsage().rss/1024), heapUsedKb: Math.round(process.memoryUsage().heapUsed/1024)};}
async function main(){
  const t0=process.hrtime.bigint();
  const out={mode, node:process.version, t0:new Date().toISOString()};
  if(mode==="shared"){
    const from=require(ROOT+"/dist/converters/fromZigbee");
    const to=require(ROOT+"/dist/converters/toZigbee");
    const exposes=require(ROOT+"/dist/lib/exposes");
    const modern=require(ROOT+"/dist/lib/modernExtend");
    const utils=require(ROOT+"/dist/lib/utils");
    const index=require(ROOT+"/dist/models-index.json");
    out.keys={from:Object.keys(from).length,to:Object.keys(to).length,exposes:Object.keys(exposes).length,modern:Object.keys(modern).length,utils:Object.keys(utils).length,index:Object.keys(index).length};
  } else if(mode==="root"){
    const zhc=require(ROOT+"/dist/index.js");
    out.exports=Object.keys(zhc).length;
  } else if(mode==="device"){
    const file=process.argv[3];
    const defs=require(ROOT+"/dist/devices/"+file);
    out.file=file;
    out.definitions=(defs.definitions||defs.default||[]).length;
  } else if(mode==="resolve"){
    const zhc=require(ROOT+"/dist/index.js");
    const device={
      type:"EndDevice", ieeeAddr:"0xa4c13887bf43cc0f", modelID:"TS0601",
      manufacturerName:"_TZE200_locansqn", manufacturerID:4417,
      powerSource:"Battery", endpoints:[{ID:1}],
      applicationVersion:72, stackVersion:0, hardwareVersion:1, dateCode:"", softwareBuildID:undefined,
    };
    const d=await zhc.findDefinition(device,false);
    out.found=!!d; out.model=d&&d.model; out.vendor=d&&d.vendor; out.file=d&&d.file;
  } else if(mode==="external"){
    const zhc=require(ROOT+"/dist/index.js");
    const ext=require("/etc/zigbee2mqtt/external_converters/L101Ze_z2m.js");
    const defs=ext.definitions||ext.default||ext;
    out.definitions=defs.length;
    if (defs[0]) { const p=zhc.prepareDefinition(defs[0]); out.model=p.model; out.zigbeeModel=p.zigbeeModel; out.from=p.fromZigbee.length; out.to=p.toZigbee.length; }
  }
  out.elapsedMs=Number(process.hrtime.bigint()-t0)/1e6;
  out.loaded=loadedStats(); out.mem=rss();
  console.log(JSON.stringify(out));
}
main().catch(e=>{console.error(e&&e.stack||e);process.exit(1)});
