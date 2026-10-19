local game=require 'main.entity_runtime'
local M={stage=0}
local function check(ok,label)
 print((ok and 'PASS ' or 'FAIL ')..label)
 if not ok then sys.exit(2); error(label) end
end
local function actor(name) return game.actors[name] end
local function position(name) return go.get_position(actor(name).id) end
local function place(self,p) go.set_position(p);self.vel=vmath.vector3();self.correction=vmath.vector3() end
local function ray_hits(name,p)
 local hits=physics.raycast(p+vmath.vector3(3,0,0),p-vmath.vector3(3,0,0),{hash('default')},{all=true}) or {}
 for _,h in ipairs(hits) do if h.id==actor(name).id then return true end end
 return false
end
local function begin_portal(self,name)
 self.vel=vmath.vector3(1.2,0,-3)
 self.look=vmath.quat_rotation_y(0.4)*vmath.quat_rotation_x(0.2)
 go.set_position(position(name));self.correction=vmath.vector3()
end
local function transported(source)
 local t=game.teleports[#game.teleports];check(t and t.source==source,'real portal entry '..source)
 local e=game.entities[t.source];local d=game.entities[t.destination]
 local yaw=d.transform.rotation_yxz[2]-e.transform.rotation_yxz[2]
 local expected=vmath.quat_rotation_y(yaw)
 check(vmath.length(t.after_velocity-vmath.rotate(expected,t.before_velocity))<0.0001,'portal rotates all velocity components')
 check(math.abs(vmath.length(t.after_velocity)-vmath.length(t.before_velocity))<0.0001,'portal preserves speed')
 local expected_forward=vmath.rotate(expected,vmath.rotate(t.before_look,vmath.vector3(0,0,-1)))
 check(vmath.length(vmath.rotate(t.after_look,vmath.vector3(0,0,-1))-expected_forward)<0.0001,'portal preserves full look orientation')
 check(vmath.length(go.get_position()-position(t.destination))<2,'player physically at destination')
end
function M.update(self,dt)
 local time=game.time
 if M.stage==0 and time>1 then
  check(self.check_contacts>5,'player makes real floor contact')
  check(ray_hits('func_door',position('func_door')),'closed door collider exists')
  M.closed=position('func_door');M.other=position('func_door_1')
  self.freeze=true;place(self,position('trigger_multiple'));M.stage=1
 elseif M.stage==1 and time>2 then
  check((game.events.trigger_multiple or 0)>0,'Bullet trigger_response reached first trigger')
  check(vmath.length(position('func_door')-M.closed)>2.9,'first door moved its game object')
  check(vmath.length(position('func_door_1')-M.other)<0.01,'unrelated door stayed closed')
  check(ray_hits('func_door',position('func_door')),'moved collider detected at new position')
  check(not ray_hits('func_door',M.closed),'no stale collider at closed position')
  place(self,vmath.vector3(-10,2,-14));M.stage=2
 elseif M.stage==2 and time>6 then
  check(vmath.length(position('func_door')-M.closed)<0.01,'first door closes after player exits')
  check(ray_hits('func_door',M.closed),'collider returns with closing door')
  place(self,position('func_button')+vmath.vector3(0,0.8,0));M.stage=3
 elseif M.stage==3 and time>7 then
  check((game.events.func_button or 0)>0,'Bullet button trigger_response received')
  check(vmath.length(position('func_door_1')-M.other)>2.9,'button opens second independent door')
  check(vmath.length(position('func_door')-M.closed)<0.01,'first door stays closed')
  check(ray_hits('func_door_1',position('func_door_1')),'second collider follows its mesh')
  begin_portal(self,'fps_portal');M.stage=4
 elseif M.stage==4 and time>7.3 then
  transported('fps_portal');check(#game.teleports==1,'one teleport on first entry')
  place(self,position('fps_portal_1'));M.stage=5
 elseif M.stage==5 and time>8.5 then
  check(#game.teleports==1,'arrival volume cannot bounce even after cooldown while staying inside')
  place(self,position('fps_portal_1')+vmath.vector3(0,0,4));M.stage=6
 elseif M.stage==6 and time>8.8 then
  begin_portal(self,'fps_portal_1');M.stage=7
 elseif M.stage==7 and time>9.1 then
  transported('fps_portal_1');check(#game.teleports==2,'reverse portal works after exit and re-entry')
  M.stage=8
 elseif M.stage==8 and time>10 then
  begin_portal(self,'fps_portal');M.stage=9
 elseif M.stage==9 and time>10.3 then
  transported('fps_portal');check(#game.teleports==3,'return works when arrival spawned outside sensor')
  print('GAMEPLAY_CHECK all checks passed: doors, moving colliders, triggers, bidirectional portals, orientation, velocity, no bounce')
  sys.exit(0);M.stage=10
 end
 if time>14 and M.stage<10 then sys.exit(3) end
end
return M
