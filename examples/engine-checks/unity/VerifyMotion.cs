// Run with -executeMethod VerifyMotion.Run on a disposable motion_lab project.
using System;
using System.IO;
using System.Linq;
using UnityEditor;
using UnityEditor.SceneManagement;
using UnityEngine;
public static class VerifyMotion {
    const float Dt=1f/60;
    static void Check(bool ok,string message){if(!ok)throw new Exception("MOTION_CHECK: "+message);}
    static Vector3 V(float x,float y,float z)=>new Vector3(-x,y,z);
    [Serializable] public sealed class Pose {public string mode;public double time;public float[] position,axes;}
    [Serializable] public sealed class Poses {public Pose[] poses;}
    static void PoseCheck(){
        string folder=Path.Combine(Application.dataPath,"MotionFixtures");
        var node=new GameObject("Pose check");node.transform.position=V(3,4,2);
        try {
            var animation=node.AddComponent<HeronBrushAnimation>();animation.dataJson=File.ReadAllText(Path.Combine(folder,"data.json"));
            var poses=JsonUtility.FromJson<Poses>(File.ReadAllText(Path.Combine(folder,"expected.json"))).poses;
            foreach(var pose in poses){
                Check(animation.Play(pose.mode),"pose clip");animation.Seek(pose.time);animation.Advance(0);
                Check(Vector3.Distance(node.transform.position,V(pose.position[0],pose.position[1],pose.position[2]))<.0001f,"pose position");
                for(int axis=0;axis<3;axis++){
                    float sign=axis==0?-1:1;var expected=V(pose.axes[axis*3],pose.axes[axis*3+1],pose.axes[axis*3+2])*sign;
                    var unit=axis==0?Vector3.right:axis==1?Vector3.up:Vector3.forward;
                    Check(Vector3.Distance(node.transform.rotation*unit,expected)<.00001f,"pose orientation axis");
                }
            }
            animation.Stop();Check(Vector3.Distance(node.transform.position,V(3,4,2))<.0001f,"stop restores pose");
            animation.Play("once");animation.Advance(100);Check(!animation.IsPlaying,"once stops");
            animation.Play("loop");animation.Pause();animation.Advance(1);Check(animation.ClipTime==0,"pause clock");animation.Resume();animation.Advance(.1);Check(animation.ClipTime>.09,"resume clock");
            Debug.Log("HERON_UNITY_POSES_OK: "+poses.Length+" editor poses and playback controls");
        }finally{UnityEngine.Object.DestroyImmediate(node);}
    }
    static void Step(HeronBrushMotionPlayer player,Vector3 walk=default(Vector3),bool jump=false){player.Step(Dt,walk,jump);Physics.Simulate(Dt);}
    public static void Run(){
        var mode=Physics.simulationMode;
        try {
            EditorSceneManager.NewScene(NewSceneSetup.EmptyScene,NewSceneMode.Single);
            PoseCheck();
            HeronBrushLevelImporter.Build();Physics.simulationMode=SimulationMode.Script;Physics.SyncTransforms();
            Check(Physics.Raycast(V(8,1,4.5f),Vector3.down,out var floor,2,0x7fffffff,QueryTriggerInteraction.Ignore)&&Math.Abs(floor.point.y)<.001f,"native floor top at y=0");
            var root=GameObject.Find("motion_lab");Check(root,"imported motion_lab root");
            var actors=root.transform.Cast<Transform>().ToDictionary(t=>t.name,t=>t);
            var capsule=new GameObject("Motion check player");capsule.layer=31;
            var body=capsule.AddComponent<CharacterController>();body.height=1.8f;body.radius=.35f;body.center=Vector3.up*.9f;
            body.skinWidth=.01f;body.stepOffset=.3f;body.minMoveDistance=0;
            using(var player=new HeronBrushMotionPlayer(root.transform,body)){
                Action<int> frames=n=>{for(int i=0;i<n;i++)Step(player);};
                Action reset=()=>{foreach(var a in root.GetComponentsInChildren<HeronBrushAnimation>())a.Stop();player.Place(V(16,1,12));frames(8);};
                var names=new[]{"func_animate","func_animate_1","func_animate_2"};var seconds=new[]{17,15,120};
                for(int test=0;test<names.Length;test++){
                    reset();var platform=actors[names[test]];var animation=platform.GetComponent<HeronBrushAnimation>();
                    Check(animation,"missing animation "+names[test]);player.Place(platform.position+V(test==2?1:0,.4f,0));frames(60);
                    Check(player.Grounded,names[test]+" initial floor contact");var anchor=platform.InverseTransformPoint(body.transform.position);
                    animation.Resume();float drift=0,vertical=0;int airborne=0;bool turned=false,cut=false;var lo=platform.position;var hi=lo;
                    for(int i=0;i<seconds[test]*60;i++){
                        Step(player);var d=platform.InverseTransformPoint(body.transform.position)-anchor;
                        drift=Math.Max(drift,new Vector2(d.x,d.z).magnitude);vertical=Math.Max(vertical,Math.Abs(d.y));if(!player.Grounded)airborne++;
                        lo=Vector3.Min(lo,platform.position);hi=Vector3.Max(hi,platform.position);turned|=Math.Abs(platform.rotation.y)>.4f;
                        cut|=test==0&&platform.position.x>4.04f&&platform.position.z>-5.96f;
                    }
                    Debug.Log(names[test]+": drift="+drift+", vertical="+vertical+", airborne="+airborne+", travel="+(hi-lo));
                    Check(drift<.16f&&vertical<.12f,names[test]+" support drift");Check(airborne<=2,names[test]+" floor contact");
                    if(test==0)Check(hi.x-lo.x>5.9f&&hi.z-lo.z>5.9f&&!cut,"shuttle L path");
                    if(test==1)Check(hi.y-lo.y>5.9f,"lift six metres");
                    if(test==2){
                        Check(turned&&drift<.01f,"rotor idle correction");var before=platform.InverseTransformPoint(body.transform.position);
                        for(int i=0;i<10;i++){var direction=platform.position-body.transform.position;direction.y=0;Step(player,direction.normalized);}
                        Check((platform.InverseTransformPoint(body.transform.position)-before).magnitude>.4f,"walk on rotor");
                    }
                    float beforeY=body.transform.position.y;Step(player,Vector3.zero,true);frames(11);
                    Check(!player.Grounded&&body.transform.position.y>beforeY+.4f,names[test]+" jump detaches");
                }
                reset();var door=actors["func_animate_3"].GetComponent<HeronBrushAnimation>();player.Place(V(8,.02f,3));frames(35);
                Check(door.IsPlaying,"door pad");frames(100);Check(Math.Abs(Math.Abs(door.transform.rotation.y)-Math.Sqrt(.5))<.02,"door quarter turn");
                player.Place(V(8,.1f,4.5f));for(int i=0;i<35;i++)Step(player,Vector3.forward);
                float openZ=body.transform.position.z;
                if(openZ<=6.5f){
                    Debug.Log("Door pose "+door.transform.position+" / "+door.transform.rotation+" body "+body.transform.position);
                    foreach(var hit in Physics.RaycastAll(body.transform.position+Vector3.up,Vector3.forward,4,0x7fffffff,QueryTriggerInteraction.Ignore))Debug.Log("Passage ray "+hit.collider.name+" at "+hit.point);
                }
                Check(openZ>6.5f,"open door passage "+openZ);
                door.Stop();frames(5);player.Place(V(8,.1f,4.5f));for(int i=0;i<35;i++)Step(player,Vector3.forward);
                float closedZ=body.transform.position.z;Check(closedZ<5.8f,"closed door passage "+closedZ);
                var lift=actors["func_animate_1"].GetComponent<HeronBrushAnimation>();lift.Stop();frames(5);player.Place(V(6,.02f,-3));frames(10);Check(lift.IsPlaying,"lift pad");
                Debug.Log("HERON_MOTION_LAB_UNITY_OK: open z="+openZ+", closed z="+closedZ);
            }
            UnityEngine.Object.DestroyImmediate(capsule);EditorApplication.Exit(0);
        }catch(Exception error){Debug.LogException(error);EditorApplication.Exit(2);}
        finally{Physics.simulationMode=mode;}
    }
}
