// Shared runtime for the validated Heron Brush animation payload.
// Flax uses centimetres and reflects the exported Godot frame across Z.
using System;
using System.Collections.Generic;
using System.Linq;
using FlaxEngine;
using Newtonsoft.Json;

namespace Game
{
    public sealed class HeronBrushAnimation : Script
    {
        public string DataJson;
        [NoSerialize,HideInEditor] public bool ManualUpdate;
        public sealed class Key { public double time,value,incoming,outgoing; public string interpolation; }
        public sealed class Channel { public Key[] keys; }
        public sealed class Clip { public string name,mode; public double duration; public Channel[] channels; }
        public sealed class Payload { public uint version; public double scale; public double[] pivot; public string selected; public bool autoplay; public Clip[] clips; }
        private Payload _data;
        private Clip _clip;
        private double _time;
        private bool _playing, _posed;
        private Vector3 _restPosition, _pivot;
        private Quaternion _restRotation;
        public bool IsPlaying => _playing;
        public string CurrentClip => _clip?.name;
        public double ClipTime => _time;
        private static bool Finite(double value) => !double.IsNaN(value) && !double.IsInfinity(value);
        private void Ensure()
        {
            if (_data != null) return;
            var data=JsonConvert.DeserializeObject<Payload>(DataJson);
            if(data==null || data.version!=1 || !Finite(data.scale) || data.scale<=0 || data.pivot==null || data.pivot.Length!=3 ||
               data.pivot.Any(v=>!Finite(v)) || data.clips==null || data.clips.Length==0 ||
               data.clips.Any(c=>c.channels==null || c.channels.Length!=6 || !Finite(c.duration) || c.duration<=0))
                throw new InvalidOperationException("Invalid Heron Brush animation payload");
            _clip=data.clips.FirstOrDefault(c=>c.name==data.selected) ?? throw new InvalidOperationException("Missing selected clip");
            _data=data;_restPosition=Actor.LocalPosition;_restRotation=Actor.LocalOrientation;
            _pivot=Convert(data.pivot[1],data.pivot[2],data.pivot[0],100/data.scale);
            _playing=_posed=data.autoplay;
        }
        private static Vector3 Convert(double x,double y,double z,double scale) => new Vector3((float)(x*scale),(float)(y*scale),(float)(-z*scale));
        public override void OnStart() {Ensure();Apply();}
        public override void OnFixedUpdate() {if(!ManualUpdate)Advance(Time.DeltaTime);}
        public bool Play(string name=null)
        {
            Ensure();var clip=_data.clips.FirstOrDefault(c=>c.name==(name??_data.selected));
            if(clip==null)return false;_clip=clip;_time=0;_playing=_posed=true;Apply();return true;
        }
        public void Activate()
        {
            Ensure();
            if(_data.clips.Any(c=>c.name=="open") && _data.clips.Any(c=>c.name=="close"))
            {
                var next=_posed&&_clip.name=="open"?"close":"open";
                var reverse=_posed&&_playing;var phase=Math.Min(_time/_clip.duration,1);
                Play(next);if(reverse)Seek((1-phase)*_clip.duration);
            }
            else Play();
        }
        public void Pause() {_playing=false;}
        public void Resume() {Ensure();_playing=_posed=true;}
        public void Stop() {Ensure();_playing=_posed=false;_time=0;Apply();}
        public void Seek(double seconds) {Ensure();if(Finite(seconds)){_time=Math.Max(0,seconds);_posed=true;Apply();}}
        public void Advance(double dt)
        {
            Ensure();if(!Finite(dt)||dt<0)throw new ArgumentOutOfRangeException(nameof(dt));
            if(_playing)
            {
                _time+=dt;
                if(_clip.mode=="once"&&_time>=_clip.duration){_time=_clip.duration;_playing=false;}
                else if(_clip.mode!="once")_time%=_clip.duration*(_clip.mode=="pingpong"?2:1);
            }
            Apply();
        }
        public static double Sample(Channel channel,double time)
        {
            var keys=channel.keys;if(keys.Length==0)return 0;if(time<=keys[0].time)return keys[0].value;
            int lo=0,hi=keys.Length;while(lo<hi){int m=lo+(hi-lo)/2;if(keys[m].time<=time)lo=m+1;else hi=m;}
            var a=keys[lo-1];if(lo==keys.Length||a.interpolation=="hold")return a.value;
            var b=keys[lo];double duration=b.time-a.time,f=(time-a.time)/duration,u=1-f;
            if(a.interpolation=="bezier")return u*u*u*a.value+3*u*u*f*(a.value+a.outgoing*duration/3)+3*u*f*f*(b.value-b.incoming*duration/3)+f*f*f*b.value;
            return a.value+(b.value-a.value)*f;
        }
        private void Apply()
        {
            if(!_posed){Actor.LocalPosition=_restPosition;Actor.LocalOrientation=_restRotation;return;}
            double t=_clip.mode=="once"?Math.Min(_time,_clip.duration):_time%(_clip.duration*(_clip.mode=="pingpong"?2:1));
            if(t>_clip.duration)t=2*_clip.duration-t;
            var v=_clip.channels.Select(channel=>Sample(channel,t)).ToArray();
            const double radians=Math.PI/180;
            var q=Quaternion.RotationAxis(Float3.UnitY,(float)(-v[5]*radians))*
                  Quaternion.RotationAxis(Float3.UnitX,(float)(-v[4]*radians))*
                  Quaternion.RotationAxis(Float3.UnitZ,(float)(v[3]*radians));
            Actor.LocalPosition=Vector3.Transform(_restPosition-_pivot,q)+_pivot+Convert(v[1],v[2],v[0],100/_data.scale);
            Actor.LocalOrientation=q*_restRotation;
        }
    }

    // Attach to a player's feet actor and assign the imported level root.
    public sealed class HeronBrushActivator : Script
    {
        public Actor LevelRoot;
        internal static readonly HashSet<HeronBrushActivator> Instances=new HashSet<HeronBrushActivator>();
        public override void OnEnable() {Instances.Add(this);}
        public override void OnDisable() {Instances.Remove(this);}
    }

    public sealed class HeronBrushAnimationTrigger : Script
    {
        public string DataJson;
        [NoSerialize,HideInEditor] public bool ManualUpdate;
        public Actor LevelRoot;
        public Actor[] Targets=Array.Empty<Actor>();
        private sealed class Payload {public string action,clip;public bool once;public double wait;public double[] origin;public double[][][] volumes;}
        private Payload _data;
        private HashSet<Guid> _inside=new HashSet<Guid>();
        private bool _fired,_pending;
        private double _wait;
        public void Activate() {_pending=true;}
        public override void OnFixedUpdate()
        {
            if(ManualUpdate)return;
            var actors=HeronBrushActivator.Instances.Where(a=>a!=null && a.Actor!=null && a.LevelRoot==LevelRoot)
                .ToDictionary(a=>a.ID,a=>a.Actor.Position);
            Advance(Time.DeltaTime,actors);
        }
        public void Advance(double dt,IReadOnlyDictionary<Guid,Vector3> actors)
        {
            if(_data==null)_data=JsonConvert.DeserializeObject<Payload>(DataJson);
            _wait=Math.Max(0,_wait-dt);var inside=new HashSet<Guid>();
            foreach(var actor in actors)
            {
                var p=Actor.Transform.WorldToLocal(actor.Value);var o=_data.origin;
                if(_data.volumes.Any(planes=>planes.Length>0 && planes.All(v=>
                    v[0]*(p.X/100+o[0])+v[1]*(p.Y/100+o[1])+v[2]*(-p.Z/100+o[2])<=v[3]+1e-5)))inside.Add(actor.Key);
            }
            var entered=inside.Any(id=>!_inside.Contains(id));_inside=inside;
            var requested=_pending||entered;_pending=false;
            if(!requested||_wait>0||(_data.once&&_fired))return;
            _fired=true;_wait=_data.wait;
            foreach(var target in Targets)
            {
                var animation=target?.GetScript<HeronBrushAnimation>();if(animation==null)continue;
                switch(_data.action)
                {
                    case "play":animation.Play(string.IsNullOrEmpty(_data.clip)?null:_data.clip);break;
                    case "open":case "close":animation.Play(_data.action);break;
                    case "pause":animation.Pause();break;
                    case "resume":animation.Resume();break;
                    case "stop":animation.Stop();break;
                    case "":case "toggle":case null:animation.Activate();break;
                }
            }
        }
    }
}
