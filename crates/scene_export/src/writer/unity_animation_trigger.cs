// Generated links from map targets. Activate() can also be called by interaction code.
using System;
using System.Collections.Generic;
using UnityEngine;
public sealed class HeronBrushAnimationTrigger:MonoBehaviour {
    [Serializable] public sealed class Data {public bool once,button;public double wait;public string action,clip;}
    public string dataJson;
    public HeronBrushAnimation[] targets;
    [NonSerialized] public bool ManualUpdate;
    Data data;bool fired;double next,clock;
    HashSet<int> inside=new HashSet<int>();
    public bool Activate(){double now=ManualUpdate?clock:Time.timeAsDouble;if(data==null)data=JsonUtility.FromJson<Data>(dataJson);if((data.once&&fired)||now<next)return false;bool delivered=false;foreach(var target in targets)if(target){switch(data.action){case "play":delivered=target.Play(string.IsNullOrEmpty(data.clip)?null:data.clip)||delivered;break;case "open":case "close":delivered=target.Play(data.action)||delivered;break;case "pause":target.Pause();delivered=true;break;case "resume":target.Resume();delivered=true;break;case "stop":target.Stop();delivered=true;break;case null:case "":case "toggle":target.Activate();delivered=true;break;}}if(delivered){fired=true;next=now+data.wait;}return delivered;}
    public void Advance(double dt,IReadOnlyDictionary<int,Vector3> feet){
        if(double.IsNaN(dt)||double.IsInfinity(dt)||dt<0)throw new ArgumentOutOfRangeException(nameof(dt));
        clock+=dt;var current=new HashSet<int>();var colliders=GetComponentsInChildren<Collider>();
        foreach(var actor in feet)foreach(var collider in colliders)
            if(collider.enabled&&collider.isTrigger&&(collider.ClosestPoint(actor.Value)-actor.Value).sqrMagnitude<1e-8f){current.Add(actor.Key);break;}
        bool entered=false;foreach(int id in current)entered|=!inside.Contains(id);inside=current;
        if(entered)Activate();
    }
    void OnTriggerEnter(Collider other){if(!ManualUpdate&&(other.GetComponentInParent<CharacterController>() || other.CompareTag("Player")))Activate();}
    void OnCollisionEnter(Collision other){if(!ManualUpdate&&(other.collider.GetComponentInParent<CharacterController>() || other.collider.CompareTag("Player")))Activate();}
}
