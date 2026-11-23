import json,re
p='.upstream/zigbee2mqtt/node_modules/zigbee-herdsman-converters/dist/devices/tuya.js'
s=open(p).read()
marker='exports.definitions = ['
m=s.index(marker); start=m+len(marker)-1
i=start
# states: code, sq, dq, tpl, line, block; template expression stack
mode='code'; tpl_expr=[]; brace=0; arrdepth=0
obj_starts=[]; obj_depth=0
while i<len(s):
 c=s[i]; n=s[i+1] if i+1<len(s) else ''
 if mode=='line':
  if c=='\n': mode='code'
 elif mode=='block':
  if c=='*' and n=='/': mode='code'; i+=1
 elif mode in ('sq','dq'):
  if c=='\\': i+=1
  elif (mode=='sq' and c=="'") or (mode=='dq' and c=='"'): mode='code'
 elif mode=='tpl':
  if c=='\\': i+=1
  elif c=='`': mode='code'
  elif c=='$' and n=='{': tpl_expr.append(0); mode='code'; i+=1
 elif mode=='code':
  if c=='/' and n=='/': mode='line'; i+=1
  elif c=='/' and n=='*': mode='block'; i+=1
  elif c=="'": mode='sq'
  elif c=='"': mode='dq'
  elif c=='`': mode='tpl'
  elif c=='{':
   if tpl_expr: tpl_expr[-1]+=1
   elif arrdepth==1 and obj_depth==0: obj_starts.append(i)
   obj_depth+=1
  elif c=='}':
   if tpl_expr:
    if tpl_expr[-1]==0: tpl_expr.pop(); mode='tpl'
    else: tpl_expr[-1]-=1
   else:
    obj_depth-=1
    if obj_depth<0: break
  elif c=='[':
   if tpl_expr: pass
   else: arrdepth+=1
  elif c==']':
   if tpl_expr: pass
   else:
    arrdepth-=1
    if arrdepth==0: end=i; break
 i+=1
# compute matching end for each top-level object (scan again simple with same scanner? use starts and find braces)
# re-scan to pair objects at arrdepth==1
pairs=[]; stack=[]; mode='code'; tpl_expr=[]; arrdepth=0; obj_depth=0; i=start
while i<len(s):
 c=s[i]; n=s[i+1] if i+1<len(s) else ''
 if mode=='line':
  if c=='\n': mode='code'
 elif mode=='block':
  if c=='*' and n=='/': mode='code'; i+=1
 elif mode in ('sq','dq'):
  if c=='\\': i+=1
  elif (mode=='sq' and c=="'") or (mode=='dq' and c=='"'): mode='code'
 elif mode=='tpl':
  if c=='\\': i+=1
  elif c=='`': mode='code'
  elif c=='$' and n=='{': tpl_expr.append(0); mode='code'; i+=1
 elif mode=='code':
  if c=='/' and n=='/': mode='line'; i+=1
  elif c=='/' and n=='*': mode='block'; i+=1
  elif c=="'": mode='sq'
  elif c=='"': mode='dq'
  elif c=='`': mode='tpl'
  elif c=='{':
   if tpl_expr: tpl_expr[-1]+=1
   else:
    obj_depth+=1
    if arrdepth==1 and obj_depth==1: stack.append(i)
  elif c=='}':
   if tpl_expr:
    if tpl_expr[-1]==0: tpl_expr.pop(); mode='tpl'
    else: tpl_expr[-1]-=1
   else:
    obj_depth-=1
    if arrdepth==1 and obj_depth==0 and stack:
     pairs.append((stack.pop(), i)); 
    if obj_depth<0: break
  elif c=='[':
   if not tpl_expr: arrdepth+=1
  elif c==']':
   if not tpl_expr:
    arrdepth-=1
    if arrdepth==0: end=i; break
 i+=1
sizes=[b-a+1 for a,b in pairs]
total=sum(sizes)
preamble=s[:start]
print(json.dumps({
 'fileBytes':len(s),'preambleBytes':len(preamble),'arrayStart':start,'arrayEnd':end,'arrayBytes':end-start+1,
 'definitions':len(pairs),'definitionBytesTotal':total,'minDef':min(sizes),'medianDef':sorted(sizes)[len(sizes)//2],'maxDef':max(sizes),
 'tailBytes':len(s)-end-1,
 'exactTextPreserved': ''.join(s[a:b+1] for a,b in pairs)==s[pairs[0][0]:pairs[-1][1]+1],
},indent=2))
