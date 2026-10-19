-- Optional shared registry. Query exact convex volumes from any game controller.
local M={volumes={}}
function M.add(id,data) assert(data.version==1,'Unsupported water payload');M.volumes[id]=data end
function M.remove(id) M.volumes[id]=nil end
function M.sample(world)
 local best,depth
 for id,data in pairs(M.volumes) do
  if go.exists(id) then
   local matrix=go.get_world_transform(id);local p=vmath.inv(matrix)*vmath.vector4(world.x,world.y,world.z,1)
   for _,volume in ipairs(data.volumes) do
    local inside=#volume.planes>=4
    for _,v in ipairs(volume.planes) do if v.x*p.x+v.y*p.y+v.z*p.z>v.d+0.00001 then inside=false;break end end
    local scale=vmath.length(vmath.vector3(matrix.c1.x,matrix.c1.y,matrix.c1.z));local d=(volume.surface-p.y)*scale
    if inside and d>=0 and (not best or data.priority>best.priority or (data.priority==best.priority and d>depth)) then best=data;depth=d end
   end
  end
 end
 return best,depth
end
function M.velocity(data,current,walk,rise,dive,dt)
 local wish=vmath.vector3(walk);if vmath.length_sqr(wish)>1 then wish=vmath.normalize(wish) end
 wish=wish*data.swim_speed;wish.y=(rise and data.rise_speed or 0)-(dive and data.rise_speed or 0)-((not rise and not dive) and data.sink_speed or 0)
 return current+(wish-current)*(1-math.exp(-data.drag*dt))
end
return M
