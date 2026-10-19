-- Game-owned factories over the complete export contract.
local M={entities={},actors={},time=0,blocks={},events={},teleports={}}
local data=json.decode(assert(sys.load_resource('/main/arena.json')))
M.scale=data.scale
for _,e in ipairs(data.entities) do M.entities[e.name]=e end
function M.value(e,k,d) local p=e.properties[k];if p then return p.value end;return d end
function M.vec(v) return vmath.vector3(v[1],v[2],v[3]) end
function M.rotation(e)
 local r=e.transform.rotation_yxz
 return vmath.quat_rotation_y(r[2])*vmath.quat_rotation_x(r[1])*vmath.quat_rotation_z(r[3])
end
local function direction(s)
 local n={};for x in s:gmatch('[^ ]+') do n[#n+1]=tonumber(x) end
 local d=vmath.vector3(n[2] or 0,n[3] or 1,n[1] or 0)
 return vmath.length_sqr(d)>0 and vmath.normalize(d) or vmath.vector3(0,1,0)
end
local factories={}
function factories.func_door(self,e)
 self.closed=go.get_position();self.open=self.closed+direction(M.value(e,'open_direction','0 0 1'))*M.value(e,'travel',96)/M.scale
 self.speed=M.value(e,'speed',100)/M.scale;self.wait=M.value(e,'wait',3);self.holders={};self.held=0;self.opening=false
end
function M.init(self,name)
 self.entity=M.entities[name];self.name=name;M.actors[name]={id=go.get_id(),url=msg.url(),state=self}
 local factory=factories[self.entity.classname];if factory then factory(self,self.entity) end
end
function M.update(self,dt)
 if not self.closed then return end
 local target=self.opening and self.open or self.closed
 local p=go.get_position();local delta=target-p;local distance=vmath.length(delta)
 go.set_position(distance<=self.speed*dt and target or p+delta*(self.speed*dt/distance))
 if self.opening and distance<0.001 and not next(self.holders) and self.wait>0 then
  self.held=self.held+dt;if self.held>=self.wait then self.opening=false;self.held=0 end
 end
end
function M.message(self,id,m)
 if id==hash('activate') and self.closed then
  if m.enter then self.holders[m.source]=true;self.opening=true;self.held=0 else self.holders[m.source]=nil end
 elseif id==hash('trigger_response') and m.other_id==hash('/player/body') then
  local e=self.entity
  if e.classname=='fps_portal' then
   local block=M.blocks[m.other_id]
   if not m.enter then
    if block and block.destination==self.name then block.destination=nil end
    return
   end
   if block and M.time<block.until_time then block.occupied=true;return end
   if block and block.destination==self.name and block.occupied then return end
   local target=M.entities[(e.targets or {})[1]];if not target then return end
   local delta=M.rotation(target)*vmath.conj(M.rotation(e))
   local pos=M.vec(target.transform.translation)+vmath.rotate(M.rotation(target),vmath.vector3(0,0,-1))*1.6
   M.blocks[m.other_id]={destination=target.name,until_time=M.time+0.6,occupied=false}
   M.events[self.name]=(M.events[self.name] or 0)+1
   msg.post('/player/body#player','portal_transport',{position=pos,delta=delta,source=self.name,destination=target.name})
  elseif e.classname=='trigger_multiple' or e.classname=='func_button' then
   if m.enter then M.events[self.name]=(M.events[self.name] or 0)+1 end
   for _,name in ipairs(e.targets or {}) do
    local target=M.actors[name];if target then msg.post(target.url,'activate',{body=m.other_id,enter=m.enter,source=self.name}) end
   end
  end
 end
end
return M
