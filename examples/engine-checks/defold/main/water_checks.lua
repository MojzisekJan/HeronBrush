local water=require 'main.arena_water'
local M={}
local function check(ok,text) if not ok then print('WATER_CHECK FAIL '..text);sys.exit(2);error(text) end end
local function place(self,p) go.set_position(p);self.vel=vmath.vector3();self.grounded=false;self.support=nil;self.keys={} end
function M.before(self,dt)
 self.water_tick=(self.water_tick or 0)+1;local n=self.water_tick
 if n==10 then
  local wet=water.sample(vmath.vector3(0,-2,0));check(wet and wet.priority==10,'overlap priority')
  check(not water.sample(vmath.vector3(0,1,0)),'above surface');check(not water.sample(vmath.vector3(6,-2,0)),'outside hull')
  place(self,vmath.vector3(0,-3,0));self.keys[hash('key_space')]=true
 elseif n==35 then check(go.get_position().y> -2.7,'rise');check(self.in_water,'swim state');place(self,vmath.vector3(0,-2,0));self.keys[hash('key_lctrl')]=true
 elseif n==60 then check(go.get_position().y < -2.3,'dive');place(self,vmath.vector3(3,-2,0))
 elseif n==85 then check(go.get_position().y < -2.1,'idle sink');place(self,vmath.vector3(3,-2,0));self.look=vmath.quat();self.keys[hash('key_w')]=true
 elseif n==110 then check(go.get_position().z<-.3,'horizontal swimming');place(self,vmath.vector3(8,1,0))
 elseif n==120 then check(not self.in_water,'dry state');print('WATER_CHECK Defold PASS overlap swim rise dive sink exit native_controller');sys.exit(0)
 end
end
return M
