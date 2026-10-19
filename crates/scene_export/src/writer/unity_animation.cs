// Generated runtime; attach through Heron Brush > Build Level. Re-export overwrites.
using System;
using UnityEngine;
public sealed class HeronBrushAnimation : MonoBehaviour {
    [TextArea] public string dataJson;
    [Serializable] public sealed class Key { public double time,value,incoming,outgoing; public string interpolation; }
    [Serializable] public sealed class Channel { public Key[] keys; }
    [Serializable] public sealed class Clip { public string name,mode; public double duration; public Channel[] channels; }
    [Serializable] public sealed class Data { public int version; public double scale; public double[] pivot; public string selected; public bool autoplay; public Clip[] clips; }
    Data data; Clip clip; Vector3 restPosition,pivot; Quaternion restRotation; Rigidbody body;
    double time; bool posed,playing,initialized;
    public bool IsPlaying { get { return playing; } }
    [NonSerialized] public bool ManualUpdate;
    public double ClipTime { get { return time; } }
    public string CurrentClip { get { return clip == null ? "" : clip.name; } }
    void Ensure() {
        if(initialized)return;
        data=JsonUtility.FromJson<Data>(dataJson);
        if(data==null||data.version!=1||data.clips==null||data.clips.Length==0)throw new InvalidOperationException("Invalid Heron animation data");
        restPosition=transform.localPosition;restRotation=transform.localRotation;
        pivot=new Vector3((float)(-data.pivot[1]/data.scale),(float)(data.pivot[2]/data.scale),(float)(data.pivot[0]/data.scale));
        body=GetComponent<Rigidbody>();if(body!=null){body.isKinematic=true;body.useGravity=false;}
        foreach(var c in data.clips)if(c.name==data.selected)clip=c;
        if(clip==null)clip=data.clips[0];initialized=true;playing=posed=data.autoplay;
    }
    void Start(){Ensure();Apply(false);}
    public bool Play(string name=null){Ensure();foreach(var c in data.clips)if(c.name==(name??data.selected)){clip=c;time=0;posed=true;playing=true;Apply(false);return true;}return false;}
    public void Activate(){Ensure();bool hasOpen=false,hasClose=false;foreach(var c in data.clips){hasOpen|=c.name=="open";hasClose|=c.name=="close";}if(hasOpen&&hasClose){string next=posed&&clip.name=="open"?"close":"open";double phase=posed&&clip!=null?Math.Min(time/clip.duration,1):1;bool reverse=posed&&playing;Play(next);if(reverse)Seek((1-phase)*clip.duration);}else Play();}
    public void Pause(){playing=false;}
    public void Resume(){Ensure();playing=true;posed=true;}
    public void Stop(){Ensure();playing=false;posed=false;time=0;Apply(false);}
    public void Seek(double seconds){Ensure();if(double.IsNaN(seconds)||double.IsInfinity(seconds))return;time=Math.Max(0,seconds);posed=true;Apply(false);}
    void FixedUpdate(){if(!ManualUpdate)Advance(Time.fixedDeltaTime,true);}
    public void Advance(double dt,bool physics=false){Ensure();if(double.IsNaN(dt)||double.IsInfinity(dt)||dt<0)throw new ArgumentOutOfRangeException(nameof(dt));if(playing){time+=dt;if(clip.mode=="once"&&time>=clip.duration){time=clip.duration;playing=false;}else if(clip.mode!="once")time%=clip.duration*(clip.mode=="pingpong"?2:1);}Apply(physics);}
    public static double Sample(Channel channel,double t) {
        var keys=channel.keys;if(keys.Length==0)return 0;if(t<=keys[0].time)return keys[0].value;
        int lo=0,hi=keys.Length;while(lo<hi){int m=lo+(hi-lo)/2;if(keys[m].time<=t)lo=m+1;else hi=m;}
        var a=keys[lo-1];if(lo==keys.Length||a.interpolation=="hold")return a.value;
        var b=keys[lo];double dt=b.time-a.time,f=(t-a.time)/dt,u=1-f;
        if(a.interpolation=="bezier")return u*u*u*a.value+3*u*u*f*(a.value+a.outgoing*dt/3)+3*u*f*f*(b.value-b.incoming*dt/3)+f*f*f*b.value;
        return a.value+(b.value-a.value)*f;
    }
    void Apply(bool physics) {
        Vector3 position=restPosition;Quaternion rotation=restRotation;
        if(posed){double t=clip.mode=="once"?Math.Min(time,clip.duration):time%(clip.duration*(clip.mode=="pingpong"?2:1));if(t>clip.duration)t=2*clip.duration-t;
            double x=Sample(clip.channels[0],t),y=Sample(clip.channels[1],t),z=Sample(clip.channels[2],t);
            Quaternion q=Quaternion.AngleAxis((float)-Sample(clip.channels[5],t),Vector3.up)*Quaternion.AngleAxis((float)Sample(clip.channels[4],t),Vector3.right)*Quaternion.AngleAxis((float)-Sample(clip.channels[3],t),Vector3.forward);
            position=q*(restPosition-pivot)+pivot+new Vector3((float)(-y/data.scale),(float)(z/data.scale),(float)(x/data.scale));rotation=q*restRotation;
        }
        if(body!=null){var parent=transform.parent;var worldPosition=parent?parent.TransformPoint(position):position;var worldRotation=parent?parent.rotation*rotation:rotation;
            if(physics){body.MovePosition(worldPosition);body.MoveRotation(worldRotation);}else{body.position=worldPosition;body.rotation=worldRotation;transform.localPosition=position;transform.localRotation=rotation;}
        }else{transform.localPosition=position;transform.localRotation=rotation;}
    }
}
