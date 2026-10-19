// Optional game-owned walker over the exported animation/collision contract.
// Use a CharacterController with Base origin, and call Step from a fixed tick.
using System;
using System.Collections.Generic;
using System.Linq;
using FlaxEngine;

namespace Game
{
    public sealed class HeronBrushMotionPlayer : IDisposable
    {
        public readonly CharacterController Body;
        public Vector3 Velocity;
        public bool Grounded {get;private set;}
        private readonly HeronBrushAnimation[] _animations;
        private readonly HeronBrushWater[] _waters;
        public HeronBrushWater Water {get;private set;}
        public bool Underwater {get;private set;}
        private readonly HeronBrushAnimationTrigger[] _triggers;
        private readonly Dictionary<Script,bool> _manual=new Dictionary<Script,bool>();
        private Dictionary<Actor,Transform> _previous=new Dictionary<Actor,Transform>();
        public HeronBrushMotionPlayer(IReadOnlyDictionary<string,Actor> actors,CharacterController body)
        {
            Body=body;_waters=actors.Values.Select(a=>a.GetScript<HeronBrushWater>()).Where(a=>a!=null).ToArray();
            _animations=actors.Values.Select(a=>a.GetScript<HeronBrushAnimation>()).Where(a=>a!=null).ToArray();
            _triggers=actors.Values.Select(a=>a.GetScript<HeronBrushAnimationTrigger>()).Where(a=>a!=null).ToArray();
            foreach(var animation in _animations){_manual[animation]=animation.ManualUpdate;animation.ManualUpdate=true;}
            foreach(var trigger in _triggers){_manual[trigger]=trigger.ManualUpdate;trigger.ManualUpdate=true;}
        }
        public void Dispose()
        {
            foreach(var animation in _animations)if(animation!=null)animation.ManualUpdate=_manual[animation];
            foreach(var trigger in _triggers)if(trigger!=null)trigger.ManualUpdate=_manual[trigger];
        }
        public void Place(Vector3 feet)
        {
            Body.Position=feet;Velocity=Vector3.Zero;Grounded=false;Physics.FlushRequests();
        }
        public void Step(float dt,Vector3 walk=default(Vector3),bool jump=false,bool dive=false)
        {
            if(float.IsNaN(dt)||float.IsInfinity(dt)||dt<0)throw new ArgumentOutOfRangeException(nameof(dt));
            var feet=new Dictionary<Guid,Vector3>{{Body.ID,Body.Position}};
            foreach(var trigger in _triggers)trigger.Advance(dt,feet);
            Actor support=null;
            // Put the player's capsule on layer 31 so the ray excludes itself;
            // exported colliders keep the game's normal collision layers.
            if(Grounded&&!jump && Physics.RayCast(Body.Position+Vector3.Up*90,Vector3.Down,out RayCastHit hit,108,0x7fffffff,false))
            {
                var actor=hit.Collider.Parent;
                while(actor!=null && actor.GetScript<HeronBrushAnimation>()==null)actor=actor.Parent;
                support=actor;
            }
            bool carry=support!=null && _previous.ContainsKey(support);
            var old=carry?_previous[support]:default(Transform);
            foreach(var animation in _animations)animation.Advance(dt);
            Physics.FlushRequests();
            if(carry)Body.Move(support.Transform.LocalToWorld(old.WorldToLocal(Body.Position))-Body.Position);
            _previous=_animations.ToDictionary(a=>a.Actor,a=>a.Actor.Transform);
            Water=HeronBrushWater.Sample(_waters,Body.Position+Vector3.Up*90,out float waterDepth);
            Underwater=HeronBrushWater.Sample(_waters,Body.Position+Vector3.Up*160,out float eyeDepth)!=null;
            if(Water!=null){var d=Water.Settings;float blend=1-(float)Math.Exp(-d.drag*dt);var wish=walk;float length=wish.Length;if(length>1)wish/=length;wish*=d.swim_speed*100;wish.Y=((jump?d.rise_speed:0)-(dive?d.rise_speed:0)-(!jump&&!dive?d.sink_speed:0))*100;Velocity=Vector3.Lerp(Velocity,wish,blend);Body.Move(Velocity*dt);Grounded=false;return;}
            Velocity.X=walk.X*500;Velocity.Z=walk.Z*500;
            if(jump&&Grounded)Velocity.Y=700;
            else if(Grounded&&Velocity.Y<0)Velocity.Y=-100;
            Velocity.Y-=2200*dt;
            Body.Move(Velocity*dt);Grounded=Body.IsGrounded;
            if(Grounded&&Velocity.Y<0)Velocity.Y=0;
        }
    }
}
