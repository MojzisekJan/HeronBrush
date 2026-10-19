-- Native motion_lab checks. Run a motion_lab export named arena with check.enabled=3.
-- Observation happens in late_update, after the actual player/physics step.
local game=require 'main.entity_runtime'
local M={stage='reset',frame=0,index=tonumber(sys.get_config_string('check.motion_start','1'))}
local cases={{name='func_animate',seconds=17,offset=vmath.vector3(0,2,0)},
 {name='func_animate_1',seconds=15,offset=vmath.vector3(0,2,0)},
 {name='func_animate_2',seconds=120,offset=vmath.vector3(1,2,0)}}
local function check(ok,label)
 print((ok and 'PASS ' or 'FAIL ')..label)
 if not ok then sys.exit(2);error(label) end
end
local function actor(name) return assert(game.actors[name],'Missing motion actor '..name) end
local function send(name,message) msg.post(actor(name).url,message) end
local function point(matrix,p)
 local v=matrix*vmath.vector4(p.x,p.y,p.z,1);return vmath.vector3(v.x,v.y,v.z)
end
local function local_position(name) return point(vmath.inv(go.get_world_transform(actor(name).id)),go.get_world_position()) end
local function place(self,p)
 go.set_position(p);self.vel=vmath.vector3();self.keys={};self.grounded=false
 self.support=nil;self.support_pose=nil;self.correction=vmath.vector3()
end
local function reset(self)
 for _,c in ipairs(cases) do send(c.name,'heron_stop_animation') end
 send('func_animate_3','heron_stop_animation')
 place(self,vmath.vector3(16,2,12));M.stage='reset';M.frame=0
end
function M.update(self,dt)
 M.tick=(M.tick or 0)+1
 M.frame=M.frame+1
 if M.stage=='reset' then
  if M.frame==1 then reset(self);M.frame=1 end
  if M.frame>=8 then
   local c=cases[M.index]
   if c then place(self,go.get_world_position(actor(c.name).id)+c.offset);M.stage='settle';M.frame=0
   else place(self,vmath.vector3(8,1.02,3));M.stage='door_pad';M.frame=0 end
  end
 elseif M.stage=='settle' and M.frame>=60 then
  if not self.grounded then print('SETTLE position='..tostring(go.get_world_position())..' velocity='..tostring(self.vel)..' platform='..tostring(go.get_world_position(actor(cases[M.index].name).id))) end
  check(self.grounded,cases[M.index].name..' initial floor contact')
  send(cases[M.index].name,'heron_resume_animation');M.stage='start_ride';M.frame=0
 elseif M.stage=='walk' then
  local d=go.get_world_position(actor('func_animate_2').id)-go.get_world_position()
  self.look=vmath.quat_rotation_y(math.atan2(-d.x,-d.z))
  self.keys[hash('key_w')]=true
 elseif M.stage=='jump' and M.frame==1 then self.want_jump=true;M.jump_y=go.get_world_position().y
 elseif M.stage=='door_pad' and M.frame==35 then
  local h=actor('func_animate_3').state.hb
  check(h.playing and h.clip.name=='open','door pad activates exported animation')
 elseif M.stage=='open_passage' then
  self.look=vmath.quat_rotation_y(math.pi);self.keys[hash('key_w')]=true
 elseif M.stage=='closed_passage' then
  self.look=vmath.quat_rotation_y(math.pi);self.keys[hash('key_w')]=true
 end
end
function M.observe(self,dt)
 if M.observed_tick==M.tick then return end
 M.observed_tick=M.tick
 local c=cases[M.index]
 if M.stage=='start_ride' then
  M.anchor=local_position(c.name);M.drift=0;M.vertical=0;M.airborne=0;M.turned=false;M.cut=false
  M.lo=go.get_world_position(actor(c.name).id);M.hi=vmath.vector3(M.lo);M.stage='ride';M.frame=0
 elseif M.stage=='ride' then
  local p=local_position(c.name);local d=p-M.anchor
  M.drift=math.max(M.drift,math.sqrt(d.x*d.x+d.z*d.z));M.vertical=math.max(M.vertical,math.abs(d.y))
  if not self.grounded then M.airborne=M.airborne+1 end
  local o=go.get_world_position(actor(c.name).id)
  for _,axis in ipairs({'x','y','z'}) do M.lo[axis]=math.min(M.lo[axis],o[axis]);M.hi[axis]=math.max(M.hi[axis],o[axis]) end
  M.turned=M.turned or math.abs(go.get_world_rotation(actor(c.name).id).y)>0.4
  M.cut=M.cut or c.name=='func_animate' and o.x < -4.04 and o.z > -5.96
  if M.frame>=c.seconds*60 then
   print(c.name..' drift='..M.drift..' vertical='..M.vertical..' airborne='..M.airborne..' travel='..tostring(M.hi-M.lo))
   check(M.drift<0.16 and M.vertical<0.12,c.name..' stays on support')
   check(M.airborne<=2,c.name..' keeps floor contact')
   if c.name=='func_animate_2' then check(M.drift<0.01,'rotor idle rotation correction') end
   if c.name=='func_animate' then check(M.hi.x-M.lo.x>5.9 and M.hi.z-M.lo.z>5.9 and not M.cut,'shuttle follows both L segments') end
   if c.name=='func_animate_1' then check(M.hi.y-M.lo.y>5.9,'lift travels six metres') end
   if c.name=='func_animate_2' then
    check(M.turned,'rotor turns');M.walk_start=local_position(c.name);M.stage='walk';M.frame=0
   else M.stage='jump';M.frame=0 end
  end
 elseif M.stage=='walk' and M.frame>=10 then
  check(vmath.length(local_position(c.name)-M.walk_start)>0.4,'player walks on rotor')
  self.keys={};M.stage='jump';M.frame=0
 elseif M.stage=='jump' and M.frame>=12 then
  if self.grounded or go.get_world_position().y<=M.jump_y+0.4 then print('JUMP grounded='..tostring(self.grounded)..' y='..go.get_world_position().y..' start='..M.jump_y..' velocity='..tostring(self.vel)) end
  check(not self.grounded and go.get_world_position().y>M.jump_y+0.4,c.name..' jump detaches');M.index=M.index+1;reset(self)
 elseif M.stage=='door_pad' and M.frame>=135 then
  check(math.abs(math.abs(go.get_world_rotation(actor('func_animate_3').id).y)-math.sqrt(0.5))<0.02,'door opens quarter turn')
  place(self,vmath.vector3(8,1.1,4.5));M.stage='open_passage';M.frame=0
 elseif M.stage=='open_passage' and M.frame>=35 then
  check(go.get_world_position().z>6.5,'open door allows passage')
  send('func_animate_3','heron_stop_animation');self.keys={};M.stage='door_close';M.frame=0
 elseif M.stage=='door_close' and M.frame>=5 then
  place(self,vmath.vector3(8,1.1,4.5));M.stage='closed_passage';M.frame=0
 elseif M.stage=='closed_passage' and M.frame>=35 then
  check(go.get_world_position().z<5.8,'closed door blocks passage')
  send('func_animate_1','heron_stop_animation');self.keys={};M.stage='lift_reset';M.frame=0
 elseif M.stage=='lift_reset' and M.frame>=5 then
  place(self,vmath.vector3(6,1.02,-3));M.stage='lift_pad';M.frame=0
 elseif M.stage=='lift_pad' and M.frame>=10 then
  check(actor('func_animate_1').state.hb.playing,'lift pad activates exported animation')
  print('HERON_MOTION_LAB_DEFOLD_OK');sys.exit(0);M.stage='done'
 end
end
return M
