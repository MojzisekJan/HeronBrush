// Optional walker for one imported level. Call Step once per fixed tick.
using System;
using System.Collections.Generic;
using UnityEngine;
public sealed class HeronBrushMotionPlayer:IDisposable {
    public readonly CharacterController Body;
    public Vector3 Velocity;
    public bool Grounded {get;private set;}
    readonly HeronBrushAnimation[] animations;
    readonly HeronBrushWater[] waters;
    public HeronBrushWater Water {get;private set;}
    public bool Underwater {get;private set;}
    readonly HeronBrushAnimationTrigger[] triggers;
    readonly Dictionary<MonoBehaviour,bool> manual=new Dictionary<MonoBehaviour,bool>();
    readonly Dictionary<Transform,Matrix4x4> previous=new Dictionary<Transform,Matrix4x4>();
    public HeronBrushMotionPlayer(Transform level,CharacterController body){
        Body=body;waters=level.GetComponentsInChildren<HeronBrushWater>();animations=level.GetComponentsInChildren<HeronBrushAnimation>();triggers=level.GetComponentsInChildren<HeronBrushAnimationTrigger>();
        foreach(var animation in animations){manual[animation]=animation.ManualUpdate;animation.ManualUpdate=true;}
        foreach(var trigger in triggers){manual[trigger]=trigger.ManualUpdate;trigger.ManualUpdate=true;}
    }
    public void Dispose(){foreach(var animation in animations)if(animation)animation.ManualUpdate=manual[animation];foreach(var trigger in triggers)if(trigger)trigger.ManualUpdate=manual[trigger];}
    public void Place(Vector3 feet){Body.enabled=false;Body.transform.position=feet;Body.enabled=true;Velocity=Vector3.zero;Grounded=false;Physics.SyncTransforms();}
    public void Step(float dt,Vector3 walk=default(Vector3),bool jump=false,bool dive=false){
        if(float.IsNaN(dt)||float.IsInfinity(dt)||dt<0)throw new ArgumentOutOfRangeException(nameof(dt));
        var feet=new Dictionary<int,Vector3>{{Body.GetInstanceID(),Body.transform.position}};
        foreach(var trigger in triggers)trigger.Advance(dt,feet);
        Transform support=null;
        // Put the player on layer 31 so the support ray excludes its capsule.
        if(Grounded&&!jump&&Physics.Raycast(Body.transform.position+Vector3.up*.9f,Vector3.down,out var hit,1.08f,0x7fffffff,QueryTriggerInteraction.Ignore)){
            var animation=hit.collider.GetComponentInParent<HeronBrushAnimation>();if(animation)support=animation.transform;
        }
        Matrix4x4 old=default(Matrix4x4);bool carry=support&&previous.TryGetValue(support,out old);
        foreach(var animation in animations)animation.Advance(dt);
        Physics.SyncTransforms();
        if(carry)Body.Move(support.localToWorldMatrix.MultiplyPoint3x4(old.inverse.MultiplyPoint3x4(Body.transform.position))-Body.transform.position);
        previous.Clear();foreach(var animation in animations)previous[animation.transform]=animation.transform.localToWorldMatrix;
        Water=HeronBrushWater.Sample(waters,Body.transform.position+Vector3.up*(Body.height*.5f),out float waterDepth);
        Underwater=HeronBrushWater.Sample(waters,Body.transform.position+Vector3.up*(Body.height*.9f),out float eyeDepth)!=null;
        if(Water){var d=Water.Settings;float blend=1-Mathf.Exp(-d.drag*dt);var wish=Vector3.ClampMagnitude(walk,1)*d.swim_speed;wish.y=(jump?d.rise_speed:0)-(dive?d.rise_speed:0)-(!jump&&!dive?d.sink_speed:0);Velocity=Vector3.Lerp(Velocity,wish,blend);Body.Move(Velocity*dt);Grounded=false;return;}
        Velocity.x=walk.x*5;Velocity.z=walk.z*5;
        if(jump&&Grounded)Velocity.y=7;else if(Grounded&&Velocity.y<=0)Velocity.y=-1;
        Velocity.y-=22*dt;Body.Move(Velocity*dt);Grounded=Body.isGrounded;if(Grounded&&Velocity.y<0)Velocity.y=0;
    }
}
