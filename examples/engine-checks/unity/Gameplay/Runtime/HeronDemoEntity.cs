using System;
using System.Linq;
using System.Collections.Generic;
using UnityEngine;
namespace HeronBrush.Demo {
    public class HeronDemoEntity : MonoBehaviour {
        public EntityData Data {get;private set;}
        public HeronDemoWorld World {get;private set;}
        public Vector3 FrameDelta {get;private set;}
        public int PhysicsTick {get;private set;}
        public float Health {get;private set;}
        public bool Open {get;private set;}
        Vector3 closed,opened;
        Rigidbody body;
        CharacterController enemy;
        bool spent,activating,mover,automatic;
        float holdUntil,nextAttack,nextContact,enemyFall;
        readonly List<Material> ownedMaterials=new List<Material>();
        public void Configure(HeronDemoWorld world,EntityData data) {
            World=world;Data=data;closed=transform.localPosition;
            Health=data.Number("health",data.classname=="fps_barrel"?20:75);
            mover=new[]{"func_door","fps_lift","func_shuttle"}.Contains(data.classname);
            automatic=data.classname=="func_shuttle"&&data.Number("start_on",1)!=0;
            opened=closed+data.Direction(data.classname=="func_shuttle"?"direction":"open_direction")*data.Number("travel",96)/World.UnitsPerMetre;
            if(mover) {body=gameObject.AddComponent<Rigidbody>();body.isKinematic=true;body.useGravity=false;}
            switch(data.classname) {
                case "fps_portal":
                    Trigger(new Vector3(1.3f,2,.5f),Vector3.up);
                    Visual(PrimitiveType.Cube,new Vector3(-.7f,1,0),new Vector3(.16f,2.2f,.18f),data.Tint(Color.cyan),false);
                    Visual(PrimitiveType.Cube,new Vector3(.7f,1,0),new Vector3(.16f,2.2f,.18f),data.Tint(Color.cyan),false);
                    Visual(PrimitiveType.Cube,new Vector3(0,2.1f,0),new Vector3(1.5f,.16f,.18f),data.Tint(Color.cyan),false);break;
                case "fps_enemy":
                    Visual(PrimitiveType.Capsule,Vector3.up*.8f,new Vector3(.6f,.8f,.6f),new Color(.8f,.18f,.14f),false);
                    enemy=gameObject.AddComponent<CharacterController>();enemy.height=1.6f;enemy.radius=.3f;enemy.center=Vector3.up*.8f;break;
                case "fps_barrel":Visual(PrimitiveType.Cylinder,Vector3.up*.55f,new Vector3(.7f,.55f,.7f),new Color(.7f,.3f,.08f),true);break;
                case "fps_pickup":case "fps_key":case "fps_weapon_pickup":case "fps_health":
                    Visual(PrimitiveType.Cube,Vector3.up*.35f,Vector3.one*.4f,data.classname=="fps_health"?Color.green:Color.yellow,false);
                    Trigger(Vector3.one,Vector3.up*.4f);break;
                case "fps_gate":Trigger(new Vector3(2,2,2),Vector3.up);break;
                case "fps_spawner":case "info_player_start":case "worldspawn":break;
            }
        }
        void Trigger(Vector3 size,Vector3 center) {
            if(GetComponentsInChildren<Collider>().Any(c=>c.isTrigger))return;
            var c=gameObject.AddComponent<BoxCollider>();c.size=size;c.center=center;c.isTrigger=true;
        }
        GameObject Visual(PrimitiveType type,Vector3 position,Vector3 scale,Color color,bool solid) {
            var o=GameObject.CreatePrimitive(type);o.name="Demo visual";o.transform.SetParent(transform,false);o.transform.localPosition=position;o.transform.localScale=scale;
            if(!solid) {o.GetComponent<Collider>().enabled=false;Destroy(o.GetComponent<Collider>());}
            var m=new Material(Shader.Find("Standard"));m.color=color;ownedMaterials.Add(m);o.GetComponent<Renderer>().sharedMaterial=m;return o;
        }
        void OnDestroy() {foreach(var material in ownedMaterials) if(material) Destroy(material);}
        void FixedUpdate() {
            PhysicsTick++;FrameDelta=Vector3.zero;
            if(!World || World.Paused || spent) return;
            if(mover) {
                if(automatic&&!Open&&Vector3.Distance(transform.localPosition,closed)<.001f&&World.Clock>=holdUntil) Open=true;
                var before=transform.position;
                var local=Vector3.MoveTowards(transform.localPosition,Open?opened:closed,Mathf.Max(.01f,Data.Number("speed",100))/World.UnitsPerMetre*Time.fixedDeltaTime);
                var next=transform.parent?transform.parent.TransformPoint(local):local;
                FrameDelta=next-before;
                // Reopen rather than pushing a closing door through the player.
                if(!Open&&World.Player) {
                    var playerBounds=World.Player.GetComponent<CharacterController>().bounds;
                    bool blocked=GetComponentsInChildren<Collider>().Any(c=>{
                        var bounds=c.bounds;bounds.center+=FrameDelta;return !c.isTrigger&&bounds.Intersects(playerBounds);
                    });
                    if(blocked){Open=true;holdUntil=0;FrameDelta=Vector3.zero;return;}
                }
                body.MovePosition(next);
                if(Open&&Vector3.Distance(local,opened)<.001f) {
                    float wait=Data.Number("wait",3);
                    if(holdUntil==0)holdUntil=World.Clock+Mathf.Max(0,wait);
                    if(wait>0&&World.Clock>=holdUntil){Open=false;holdUntil=World.Clock+wait;}
                }
            }
            if(Data.classname=="func_rotate"&&Data.Number("start_on",1)!=0) transform.Rotate(Data.Direction("axis"),Data.Number("speed",90)*Time.fixedDeltaTime,Space.World);
            if(enemy&&World.Player&&World.Player.Health>0) {
                var delta=World.Player.transform.position-transform.position;
                bool visible=Physics.Raycast(transform.position+Vector3.up,delta.normalized,out var hit,18,~0,QueryTriggerInteraction.Ignore)&&hit.collider.GetComponent<HeronDemoPlayer>();
                var horizontal=new Vector3(delta.x,0,delta.z);
                if(visible&&horizontal.magnitude>1.5f) {enemy.Move(horizontal.normalized*2*Time.fixedDeltaTime);transform.rotation=Quaternion.LookRotation(horizontal);}
                enemyFall=enemy.isGrounded?-2:enemyFall-20*Time.fixedDeltaTime;enemy.Move(Vector3.up*enemyFall*Time.fixedDeltaTime);
                if(visible&&delta.magnitude<12&&World.Clock>=nextAttack){World.Player.Damage(8);nextAttack=World.Clock+1;}
            }
        }
        void OnTriggerEnter(Collider other) {var player=other.GetComponentInParent<HeronDemoPlayer>();if(player) Contact(player);}
        void OnTriggerStay(Collider other) {
            if(Data.classname=="fps_gate"||Data.classname=="trigger_multiple") {var p=other.GetComponentInParent<HeronDemoPlayer>();if(p) Contact(p);}
        }
        public void Contact(HeronDemoPlayer player) {
            if(spent||World.Paused||World.Clock<nextContact)return;
            if(Data.classname=="fps_portal") {
                if(World.Clock<player.PortalReadyAt)return;
                var target=Data.targets?.FirstOrDefault();
                if(target==null||!World.Entities.TryGetValue(target,out var destination))return;
                var rotation=destination.transform.rotation*Quaternion.Euler(0,180,0);
                var turn=rotation*Quaternion.Inverse(transform.rotation*Quaternion.Euler(0,180,0));
                player.PortalReadyAt=World.Clock+.7f;
                player.Teleport(destination.transform.position+rotation*Vector3.forward*1.3f,rotation,turn*player.Velocity);return;
            }
            nextContact=World.Clock+.4f;
            switch(Data.classname) {
                case "fps_pickup":
                    var kind=Data.Text("kind","present");int value=(int)Data.Number("value",1);
                    if(kind=="health")player.Heal(value);else if(kind=="ammo")player.AddAmmo(value);else World.AddItem(kind==""?"present":kind,value);
                    Consume();break;
                case "fps_key":World.AddItem(Data.Text("key","red"),1);Consume();break;
                case "fps_weapon_pickup":player.AddAmmo(60);World.Show("Blaster ammo +60");Consume();break;
                case "fps_health":player.Heal(Data.Number("value",25));Consume();break;
                case "fps_checkpoint":player.SetCheckpoint(player.transform.position,player.transform.rotation);spent=true;break;
                case "fps_jump_pad":player.Launch(Data.Number("strength",17));break;
                case "fps_sign":World.Show(Data.Text("title")+" — "+Data.Text("text"),Data.Number("seconds",4));spent=Data.Number("once",1)!=0;break;
                case "fps_exit":if(Data.Text("needs")==""||World.Count(Data.Text("needs"))>0)World.Finish(Data.Text("message","Level complete"));else World.Show("Requires "+Data.Text("needs"));break;
                case "trigger_once":World.ActivateTargets(Data,player);spent=true;break;
                case "trigger_multiple":case "fps_gate":Activate(player);break;
            }
        }
        void Consume() {spent=true;gameObject.SetActive(false);}
        public void Activate(HeronDemoPlayer player) {
            if(spent||activating)return;
            activating=true;
            try {
                var key=Data.Text("key",Data.Text("needs_key"));
                if(key!=""&&World.Count(key)==0){World.Show("Requires key: "+key);return;}
                if(Data.classname=="fps_gate") {
                    var kind=Data.Text("needs_kind","present");
                    if(World.Count(kind)<Data.Number("needs_count",1)){World.Show("Collect "+Data.Number("needs_count",1)+" "+kind);return;}
                    spent=true;World.Show(Data.Text("message","Gate opened"));
                }
                if(mover){Open=true;holdUntil=0;}
                if(Data.classname=="fps_spawner") {StartCoroutine(Spawn());spent=true;}
                World.ActivateTargets(Data,player);
            } finally {activating=false;}
        }
        System.Collections.IEnumerator Spawn() {
            for(int i=0;i<Mathf.Clamp((int)Data.Number("count",3),0,100);i++) {
                while(World.Paused)yield return null;
                var o=new GameObject("Spawned enemy");o.transform.SetParent(World.mapRoot,false);
                o.transform.position=transform.position+new Vector3(Mathf.Cos(i*2.4f),0,Mathf.Sin(i*2.4f))*Data.Number("spread",2.5f);
                o.AddComponent<HeronDemoEntity>().Configure(World,new EntityData{name=o.name,classname="fps_enemy"});
                float until=World.Clock+Mathf.Max(.05f,Data.Number("interval",.8f));
                while(World.Clock<until)yield return null;
            }
        }
        public void Damage(float damage) {
            if(spent||!(Data.classname=="fps_enemy"||Data.classname=="fps_barrel"))return;
            Health=Mathf.Max(0,Health-Mathf.Max(0,damage));if(Health>0)return;
            spent=true;
            if(Data.classname=="fps_barrel") {
                float radius=Data.Number("radius",4),power=Data.Number("damage",55);
                var targets=new HashSet<HeronDemoEntity>();
                foreach(var c in Physics.OverlapSphere(transform.position,radius)) {var entity=c.GetComponentInParent<HeronDemoEntity>();if(entity&&entity!=this)targets.Add(entity);}
                foreach(var entity in targets)entity.Damage(power);
                if(World.Player&&Vector3.Distance(World.Player.transform.position,transform.position)<radius)World.Player.Damage(power);
            }
            World.Show(Data.classname=="fps_enemy"?"Enemy defeated":"Barrel exploded");gameObject.SetActive(false);
        }
    }
}
