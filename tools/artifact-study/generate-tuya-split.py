import os,re,json,shutil,subprocess
srcp='.upstream/zigbee2mqtt/node_modules/zigbee-herdsman-converters/dist/devices/tuya.js'
s=open(srcp).read(); marker='exports.definitions = ['
start=s.index(marker)+len(marker)-1
# use ranges computed by tuya_split approach
import importlib.util
# reimplement lightweight scanner by importing previous script output? recompute pairs via exec
ns={}
exec(open('/tmp/opencode/tuya_split.py').read().replace("print(json.dumps({", "_result=({").replace("},indent=2))", "})"),ns)
# easier: parse json from script output
out=subprocess.check_output(['python3','/tmp/opencode/tuya_split.py'],text=True)
info=json.loads(out)
# recompute pairs by scanning braces with same robust logic copied minimal: use stored starts? redo from saved script logic
# Load helper by executing a modified copy exposing pairs
code=open('/tmp/opencode/tuya_split.py').read()
code=code.replace("print(json.dumps({", "print(json.dumps({'pairs': pairs, 'info': {")
code=code.replace("},indent=2))", "}},indent=2))")
raw=json.loads(subprocess.check_output(['python3','-c',code],text=True))
pairs=raw['pairs']; info=raw['info']
preamble=s[:start]
# top-level binding names in preamble
names=set()
for m in re.finditer(r'^const\s+([A-Za-z_$][\w$]*)\s*=', preamble, re.M): names.add(m.group(1))
for m in re.finditer(r'^const\s*\{([^}]+)\}\s*=', preamble, re.M):
 for part in m.group(1).split(','):
  n=part.strip().split(':')[-1].strip()
  if re.match(r'^[A-Za-z_$][\w$]*$',n): names.add(n)
for m in re.finditer(r'^function\s+([A-Za-z_$][\w$]*)\s*\(', preamble, re.M): names.add(m.group(1))
# remove imports? all names in common, export all
common=preamble+"\nmodule.exports = {"+",".join(sorted(names))+"};\n"
# split into 8 balanced groups
sizes=[b-a+1 for a,b in pairs]; total=sum(sizes); groups=8; target=total/groups
parts=[]; cur=[]; cursz=0
for i,sz in enumerate(sizes):
 cur.append(i); cursz+=sz
 if cursz>=target and len(parts)<groups-1: parts.append(cur); cur=[]; cursz=0
if cur: parts.append(cur)
root='/tmp/opencode/tuya-split'
if os.path.isdir(root): shutil.rmtree(root)
os.makedirs(root+'/dist/devices')
for d in ('converters','lib'):
 os.symlink(os.path.abspath('.upstream/zigbee2mqtt/node_modules/zigbee-herdsman-converters/dist/'+d), root+'/dist/'+d)
os.symlink(os.path.abspath('.upstream/zigbee2mqtt/node_modules'), root+'/node_modules')
open(root+'/dist/devices/tuya-common.js','w').write(common)
meta=[]
for gi,g in enumerate(parts):
 body=','.join(s[a:b+1] for a,b in (pairs[i] for i in g))
 src="\"use strict\";\nconst {"+",".join(sorted(names))+"} = require(\"./tuya-common\");\nexports.definitions = ["+body+"];\n"
 fp=root+f'/dist/devices/tuya-{gi}.js'; open(fp,'w').write(src)
 meta.append({'file':f'tuya-{gi}.js','count':len(g),'bytes':len(src),'first':g[0],'last':g[-1]})
print(json.dumps({'info':info,'preambleBytes':len(preamble),'bindingNames':len(names),'groups':len(parts),'groupBytes':[m['bytes'] for m in meta],'groupDefs':[m['count'] for m in meta],'commonBytes':len(common)},indent=2))
open('/tmp/opencode/tuya-split-meta.json','w').write(json.dumps(meta,indent=2))
