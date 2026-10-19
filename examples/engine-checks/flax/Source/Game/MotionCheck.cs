#if FLAX_EDITOR
using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using FlaxEngine;
using Newtonsoft.Json.Linq;

namespace Game
{
    // Uses real imported MeshCollider assets and Flax's CharacterController.
    public static class MotionCheck
    {
        const float Dt=1.0f/60;
        private static void Check(bool ok,string message) {if(!ok)throw new Exception("MOTION_CHECK: "+message);}
        private static Vector3 V(double x,double y,double z) => new Vector3((float)(x*100),(float)(y*100),(float)(-z*100));
        private static Vector3 V(JToken array) => V((double)array[0],(double)array[1],(double)array[2]);
        private static void PhysicsStep() {Physics.FlushRequests();Physics.Simulate(Dt);Physics.CollectResults();}
        public static void PoseCheck()
        {
            string folder=Environment.GetEnvironmentVariable("HERON_PORTABLE_FIXTURES")??"/tmp/heron-portable-animation-check";
            var actor=new EmptyActor {Name="Animation pose check",Position=V(3,4,2)};
            Check(!Level.SpawnActor(actor),"spawn pose actor");
            try
            {
                var animation=actor.AddScript<HeronBrushAnimation>();animation.DataJson=File.ReadAllText(Path.Combine(folder,"data.json"));
                var expected=JArray.Parse(File.ReadAllText(Path.Combine(folder,"expected.json")));
                foreach(var pose in expected)
                {
                    Check(animation.Play((string)pose["mode"]),"selected clip");animation.Seek((double)pose["time"]);animation.Advance(0);
                    Check((actor.Position-V(pose["position"])).Length<0.01f,"pose position "+pose["mode"]+" at "+pose["time"]);
                    for(int axis=0;axis<3;axis++)
                    {
                        var source=pose["axes"][axis];var sign=axis==2?-1:1;
                        var target=new Vector3((float)source[0]*sign,(float)source[1]*sign,-(float)source[2]*sign);
                        var unit=axis==0?Vector3.Right:axis==1?Vector3.Up:new Vector3(0,0,1);
                        Check((Vector3.Transform(unit,actor.Orientation)-target).Length<0.00001f,"pose orientation axis "+axis);
                    }
                }
                animation.Stop();Check((actor.Position-V(3,4,2)).Length<0.01f,"stop restores pose");
                animation.Play("once");animation.Advance(100);Check(!animation.IsPlaying,"once stops");
                animation.Play("loop");animation.Pause();animation.Advance(1);Check(animation.ClipTime==0,"pause stops clock");
                animation.Resume();animation.Advance(0.25);Check(Math.Abs(animation.ClipTime-0.25)<1e-6,"resume advances clock");
                Debug.Log("HERON_FLAX_POSES_OK: "+expected.Count+" editor poses and playback controls");
            }
            finally {FlaxEngine.Object.DestroyNow(actor);}
        }
        private static void Step(HeronBrushMotionPlayer player,Vector3 walk=default(Vector3),bool jump=false) {player.Step(Dt,walk,jump);PhysicsStep();}
        public static void Run(Dictionary<string,Actor> actors)
        {
            PoseCheck();
            var body=new CharacterController {Name="Motion check player",Radius=35,Height=110,OriginMode=CharacterController.OriginModes.Base,AutoGravity=false,Layer=31,Position=V(16,1,12)};
            Check(!Level.SpawnActor(body),"spawn character controller");
            var player=new HeronBrushMotionPlayer(actors,body);
            try
            {
                Action<int> frames=n=>{for(int i=0;i<n;i++)Step(player);};
                Action reset=()=>{
                    foreach(var a in actors.Values.Select(a=>a.GetScript<HeronBrushAnimation>()).Where(a=>a!=null))a.Stop();
                    player.Place(V(16,1,12));frames(8);
                };
                var names=new[]{"func_animate","func_animate_1","func_animate_2"};
                var seconds=new[]{17,15,120};
                for(int test=0;test<names.Length;test++)
                {
                    reset();var name=names[test];var platform=actors[name];var animation=platform.GetScript<HeronBrushAnimation>();
                    Check(animation!=null,"missing animation "+name);
                    player.Place(platform.Position+V(test==2?1:0,0.4,0));frames(60);
                    Check(player.Grounded,name+" initial floor contact");
                    var anchor=platform.Transform.WorldToLocal(player.Body.Position);
                    animation.Resume();float drift=0,vertical=0;int airborne=0;bool turned=false,cut=false;
                    var lo=platform.Position;var hi=lo;
                    for(int i=0;i<seconds[test]*60;i++)
                    {
                        Step(player);var local=platform.Transform.WorldToLocal(player.Body.Position);var d=local-anchor;
                        drift=Math.Max(drift,(float)Math.Sqrt(d.X*d.X+d.Z*d.Z));vertical=Math.Max(vertical,Math.Abs(d.Y));
                        if(!player.Grounded)airborne++;
                        lo=Vector3.Min(lo,platform.Position);hi=Vector3.Max(hi,platform.Position);
                        turned|=Math.Abs(platform.Orientation.Y)>0.4f;
                        cut|=test==0 && platform.Position.X < -404 && platform.Position.Z < 596;
                    }
                    Debug.Log(name+": drift="+drift/100+" m, vertical="+vertical/100+" m, airborne="+airborne+", travel="+(hi-lo)/100);
                    Check(drift<16&&vertical<12,name+" loses support");Check(airborne<=2,name+" loses floor contact");
                    if(test==0)Check(hi.X-lo.X>590&&hi.Z-lo.Z>590&&!cut,"shuttle L path");
                    if(test==1)Check(hi.Y-lo.Y>590,"lift six metres");
                    if(test==2)
                    {
                        Check(turned&&drift<1,"rotor idle correction");var before=platform.Transform.WorldToLocal(player.Body.Position);
                        for(int i=0;i<10;i++) {var direction=platform.Position-player.Body.Position;direction.Y=0;direction.Normalize();Step(player,direction);}
                        Check((platform.Transform.WorldToLocal(player.Body.Position)-before).Length>40,"walk on rotor");
                    }
                    float beforeY=player.Body.Position.Y;Step(player,Vector3.Zero,true);frames(11);
                    Check(!player.Grounded && player.Body.Position.Y>beforeY+40,name+" jump detaches");
                }
                reset();var door=actors["func_animate_3"].GetScript<HeronBrushAnimation>();
                player.Place(V(8,0.02,3));frames(35);Check(door.IsPlaying,"door pad");frames(100);
                Check(Math.Abs(Math.Abs(door.Actor.Orientation.Y)-(float)Math.Sqrt(0.5))<0.02f,"door quarter turn");
                player.Place(V(8,0.1,4.5));for(int i=0;i<35;i++)Step(player,new Vector3(0,0,-1));
                float openZ=-player.Body.Position.Z/100;Check(openZ>6.5f,"open door passage "+openZ);
                door.Stop();frames(5);player.Place(V(8,0.1,4.5));for(int i=0;i<35;i++)Step(player,new Vector3(0,0,-1));
                float closedZ=-player.Body.Position.Z/100;Check(closedZ<5.8f,"closed door passage "+closedZ);
                var lift=actors["func_animate_1"].GetScript<HeronBrushAnimation>();lift.Stop();frames(5);
                player.Place(V(6,0.02,-3));frames(10);Check(lift.IsPlaying,"lift pad");
                Debug.Log("HERON_MOTION_LAB_FLAX_OK: open z="+openZ+", closed z="+closedZ);
                reset();
            }
            finally {player.Dispose();FlaxEngine.Object.DestroyNow(player.Body);}
        }
    }
}
#endif
