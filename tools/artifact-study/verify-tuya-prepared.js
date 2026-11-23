"use strict";
const fs=require("fs");
const zhc=require("/config/workspace/zigbee2mqtt-rs/.upstream/zigbee2mqtt/node_modules/zigbee-herdsman-converters/dist/index.js");
const orig=require("/config/workspace/zigbee2mqtt-rs/.upstream/zigbee2mqtt/node_modules/zigbee-herdsman-converters/dist/devices/tuya.js").definitions;
const meta=JSON.parse(fs.readFileSync("/tmp/opencode/tuya-split-meta.json","utf8"));
// local paths differ; use local checkout for fragments
const localRoot="/tmp/opencode/tuya-split/dist/devices/";
let frag=[]; for(const m of meta) frag=frag.concat(require(localRoot+m.file).definitions);
function norm(d){
 let exp=d.exposes;
 if(typeof exp==="function") exp="function";
 else { try{exp=JSON.parse(JSON.stringify(exp))}catch(e){exp=String(exp)} }
 return JSON.stringify({model:d.model,vendor:d.vendor,description:d.description,zigbeeModel:d.zigbeeModel,meta:d.meta,endpoint:d.endpoint,version:d.version,exposes:exp,from:(d.fromZigbee||[]).map(c=>[c.cluster,c.type]),to:(d.toZigbee||[]).map(c=>[c.key,c.endpoints]),options:(d.options||[]).map(o=>o.name),cfg:typeof d.configure,evt:typeof d.onEvent});
}
let bad=[];
for(let i=0;i<orig.length;i++){
 let a,b;
 try{a=norm(zhc.prepareDefinition(orig[i]))}catch(e){a="ERR:"+e.message}
 try{b=norm(zhc.prepareDefinition(frag[i]))}catch(e){b="ERR:"+e.message}
 if(a!==b) bad.push({i,model:orig[i]&&orig[i].model});
}
console.log(JSON.stringify({original:orig.length,fragments:frag.length,badCount:bad.length,bad:bad.slice(0,10)},null,2));
