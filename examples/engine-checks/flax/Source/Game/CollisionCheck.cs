#if FLAX_EDITOR
using System;
using System.Linq;
using System.Collections.Generic;
using FlaxEngine;
namespace Game
{
    public static class CollisionCheck
    {
        private static void Check(bool value, string message)
        { if (!value) throw new Exception("PHYSICS_CHECK: " + message); }
        private static void Step(int count)
        { Physics.FlushRequests(); for (int i=0;i<count;i++) { Physics.Simulate(1.0f/60); Physics.CollectResults(); } }
        private static RigidBody Box(Vector3 position, bool gravity)
        {
            var body = new RigidBody { Name="Physics check body", Position=position, EnableGravity=gravity, EnableSimulation=true };
            Check(!Level.SpawnActor(body), "spawn rigid body");
            var box = new BoxCollider { Position=position, Size=new Vector3(50), Name="Physics check box" };
            Check(!Level.SpawnActor(box,body), "spawn box");
            box.LocalPosition=Vector3.Zero;
            return body;
        }
        public static void ReloadOnly()
        {
            Physics.FlushRequests();
            Check(Physics.RayCast(new Vector3(-1000,200,1000),Vector3.Down,out RayCastHit floor,500,uint.MaxValue,false),"fresh process floor raycast");
            Check(Math.Abs(floor.Point.Y)<1,"fresh process floor height");
            Check(floor.Collider is MeshCollider mesh && !mesh.CollisionData.IsVirtual,"fresh process persistent collider");
            var falling=Box(new Vector3(-900,250,900),true);
            Step(180);
            Check(Math.Abs(falling.Position.Y-25)<3,"fresh process rigid body contact");
            Debug.Log("PHYSICS_CHECK fresh_process_reload=PASS floor_y="+floor.Point.Y+" body_y="+falling.Position.Y);
            FlaxEngine.Object.DestroyNow(falling);
        }
        public static void Run(Dictionary<string,Actor> actors)
        {
            var colliders=actors.Values.SelectMany(a=>a.Children).OfType<MeshCollider>().ToArray();
            Check(colliders.Length==20,"expected 20 native mesh colliders, got "+colliders.Length);
            Check(colliders.All(c=>c.CollisionData!=null&&!c.CollisionData.IsVirtual&&System.IO.File.Exists(c.CollisionData.Path)),"persistent collision assets");
            var without=HeronBrushLevel.Build(createCollisions:false);
            Check(!without.Values.Any(a=>a.Children.Any(c=>c is Collider)),"createCollisions=false");
            FlaxEngine.Object.DestroyNow(without.Values.First().Parent);
            var filtered=HeronBrushLevel.Build(includeCollision:p=>p.Classname=="worldspawn");
            Check(filtered.Values.SelectMany(a=>a.Children).OfType<Collider>().Count()==1,"includeCollision filter");
            FlaxEngine.Object.DestroyNow(filtered.Values.First().Parent);
            Physics.FlushRequests();
            Check(Physics.RayCast(new Vector3(-1000,200,1000),Vector3.Down,out RayCastHit floor,500,uint.MaxValue,false),"floor raycast");
            Check(Math.Abs(floor.Point.Y)<1,"floor height "+floor.Point);
            Check(floor.Collider.Parent==actors["worldspawn"],"raycast hit exported worldspawn");
            var moving=actors["func_door"];
            var before=moving.Transform;
            moving.Position=new Vector3(10000,300,0);
            moving.Orientation=Quaternion.Euler(15,35,20);
            moving.LocalScale=new Vector3(1.4f,0.8f,1.2f);
            var direction=Vector3.Normalize(moving.Transform.LocalToWorldVector(Vector3.Right));
            Physics.FlushRequests();
            Check(Physics.RayCast(moving.Position+direction*1000,-direction,out RayCastHit rotated,2000,uint.MaxValue,false),"rotated scaled convex raycast");
            Check(rotated.Collider.Parent==moving,"convex ray hits mover");
            Check(Math.Abs(rotated.Distance-930)<2,"nonuniform scale and rotation distance "+rotated.Distance);
            moving.Transform=before;
            var falling=Box(new Vector3(-900,250,900),true);
            int contacts=0;
            falling.CollisionEnter += hit=>contacts++;
            Step(180);
            Check(Math.Abs(falling.Position.Y-25)<3,"body rests on world at y25, actual "+falling.Position.Y);
            Check(contacts>0,"body collision event");
            Debug.Log("PHYSICS_CHECK floor="+floor.Point.Y+" body_y="+falling.Position.Y+" contacts="+contacts+" convex_distance="+rotated.Distance);
            FlaxEngine.Object.DestroyNow(falling);
            var sensor=colliders.First(c=>c.IsTrigger);
            var sensorRoot=sensor.Parent;
            var original=sensorRoot.Position;
            sensorRoot.Position+=new Vector3(50000,0,0);
            Physics.FlushRequests();
            var center=sensorRoot.Transform.LocalToWorld(sensor.CollisionData.Options.Box.Center);
            var visitor=Box(center+new Vector3(10000,0,0),false);
            int enters=0,exits=0;
            sensor.TriggerEnter += other=>enters++;
            sensor.TriggerExit += other=>exits++;
            Step(2);
            visitor.Position=center;
            Step(5);
            visitor.Position=center+new Vector3(10000,0,0);
            Step(5);
            Check(enters>0&&exits>0,"sensor enter/exit "+enters+"/"+exits);
            Check(sensor.IsTrigger,"sensor never converted to blocking collider");
            FlaxEngine.Object.DestroyNow(visitor);
            sensorRoot.Position=original;
            Step(1);
            Debug.Log("PHYSICS_CHECK colliders="+colliders.Length+" sensor_enter="+enters+" sensor_exit="+exits);
            var scene=Level.GetScene(0);var id=scene.ID;
            Check(!Level.SaveScene(scene),"save physics scene");
            Check(!Level.UnloadScene(scene),"unload physics scene");
            Check(!Level.LoadScene(id),"reload physics scene");
            Physics.FlushRequests();
            Check(Physics.RayCast(new Vector3(-1000,200,1000),Vector3.Down,out RayCastHit reloaded,500,uint.MaxValue,false),"reloaded persistent floor raycast");
            Check(reloaded.Collider is MeshCollider mesh&&!mesh.CollisionData.IsVirtual,"reloaded collision uses disk asset");
            Debug.Log("PHYSICS_CHECK persisted_scene_reload=PASS disabled=PASS filtered=PASS");
        }
    }
}
#endif
