"use strict";
const fs=require("fs");
const orig=require("/config/workspace/zigbee2mqtt-rs/.upstream/zigbee2mqtt/node_modules/zigbee-herdsman-converters/dist/devices/tuya.js").definitions;
const meta=JSON.parse(fs.readFileSync("/tmp/opencode/tuya-split-meta.json","utf8"));
let frag=[];
for(const m of meta) frag=frag.concat(require("/tmp/opencode/tuya-split/dist/devices/"+m.file).definitions);
function norm(d){return {
 model:d.model,vendor:d.vendor,description:d.description,
 zigbeeModel:d.zigbeeModel,fingerprint:d.fingerprint,whiteLabel:d.whiteLabel,
 from:(d.fromZigbee||[]).map(c=>[c.cluster,c.type]),
 to:(d.toZigbee||[]).map(c=>[c.key,c.endpoints]),
 exType:typeof d.exposes, cfg:typeof d.configure, evt:typeof d.onEvent,
 endpoint:d.endpoint,options:(d.options||[]).map(o=>o.name),
 meta:d.meta,
};}
let bad=[];
for(let i=0;i<orig.length;i++){
 const a=JSON.stringify(norm(orig[i])); const b=JSON.stringify(norm(frag[i]));
 if(a!==b) bad.push({i,model:orig[i]&&orig[i].model});
}
console.log(JSON.stringify({original:orig.length,fragments:frag.length,badCount:bad.length,bad:bad.slice(0,10)},null,2));
