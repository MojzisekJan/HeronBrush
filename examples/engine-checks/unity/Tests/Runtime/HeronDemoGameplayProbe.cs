using System;
using System.Linq;
using System.Collections;
using UnityEngine;
namespace HeronBrush.Demo {
    // Only installed by the native test runner, not needed by the playable kit.
    public class HeronDemoGameplayProbe : MonoBehaviour {
        public Action<bool,string> Completed;
        void Check(bool ok,string why) {if(!ok)throw new Exception(why);}
        IEnumerator Start() {
            yield return null;
            var steps=Run();
            while(true) {
                object next;
                try {if(!steps.MoveNext())break;next=steps.Current;}
                catch(Exception e){Completed?.Invoke(false,e.ToString());yield break;}
                yield return next;
            }
            Completed?.Invoke(true,"Native gameplay checks passed");
        }
        IEnumerator Run() {
            var world=FindFirstObjectByType<HeronDemoWorld>();world.Initialize();world.captureMouse=false;world.SetPaused(false);
            var player=world.Player;player.enabled=false;
            foreach(var e in world.Entities.Values.Where(e=>e.Data.classname=="fps_enemy"))e.enabled=false;
            Check(player&&player.View&&player.GetComponent<CharacterController>(),"Player missing");
            // Isolated physics lane avoids depending on a particular demo floor layout.
            var floor=GameObject.CreatePrimitive(PrimitiveType.Cube);floor.transform.position=new Vector3(100,0,100);floor.transform.localScale=new Vector3(20,1,20);
            player.Teleport(new Vector3(100,1,100),Quaternion.identity,Vector3.zero);
            for(int i=0;i<45;i++){player.StepMovement(Vector2.zero,false,.02f);yield return new WaitForFixedUpdate();}
            Check(player.GetComponent<CharacterController>().isGrounded,"Player did not land on collider");
            var position=player.transform.position;
            for(int i=0;i<10;i++){player.StepMovement(Vector2.up,false,.02f);yield return new WaitForFixedUpdate();}
            Check(player.transform.position.z>position.z+.5f,"Walking failed");
            var ground=player.transform.position.y;player.StepMovement(Vector2.zero,true,.02f);
            for(int i=0;i<8;i++){player.StepMovement(Vector2.zero,false,.02f);yield return new WaitForFixedUpdate();}
            Check(player.transform.position.y>ground+.3f,"Jump failed");
            Debug.Log("HERON_GAMEPLAY movement PASS");
            var door=world.Entities.Values.First(e=>e.Data.classname=="func_door");var closed=door.transform.position;
            var trigger=world.Entities.Values.First(e=>e.Data.targets!=null&&e.Data.targets.Contains(door.Data.name));
            var volume=trigger.GetComponentsInChildren<Collider>().First(c=>c.isTrigger).bounds;
            player.Teleport(volume.center-Vector3.up*.5f,Quaternion.identity,Vector3.zero);
            player.GetComponent<CharacterController>().Move(Vector3.up*.01f);
            for(int i=0;i<5;i++)yield return new WaitForFixedUpdate();
            Check(door.Open,"Entering exported trigger did not activate door");
            player.Teleport(new Vector3(100,2,100),Quaternion.identity,Vector3.zero);
            for(int i=0;i<20;i++)yield return new WaitForFixedUpdate();
            Check(Vector3.Distance(closed,door.transform.position)>.4f,"Trigger did not move door");
            for(int i=0;i<230;i++)yield return new WaitForFixedUpdate();
            Check(Vector3.Distance(closed,door.transform.position)<.05f,"Door did not close after wait");
            var button=world.Entities.Values.First(e=>e.Data.classname=="fps_use_button");button.Activate(player);
            Check(world.Entities[button.Data.targets[0]].Open,"Button target failed");
            var liftObject=GameObject.CreatePrimitive(PrimitiveType.Cube);
            liftObject.transform.position=new Vector3(110,2,110);liftObject.transform.localScale=new Vector3(3,.3f,3);
            var lift=liftObject.AddComponent<HeronDemoEntity>();
            lift.Configure(world,new EntityData{name="Test lift",classname="fps_lift",properties=new[]{new Property{name="travel",value="64"},new Property{name="speed",value="64"},new Property{name="wait",value="0"}}});
            player.Teleport(new Vector3(110,2.3f,110),Quaternion.identity,Vector3.zero);
            for(int i=0;i<20;i++){player.StepMovement(Vector2.zero,false,.02f);yield return new WaitForFixedUpdate();}
            var liftStart=player.transform.position.y;lift.Activate(player);
            for(int i=0;i<35;i++){player.StepMovement(Vector2.zero,false,.02f);yield return new WaitForFixedUpdate();}
            Check(player.transform.position.y>liftStart+.6f,"Lift did not carry player");
            Debug.Log("HERON_GAMEPLAY doors_platform PASS");
            var portal=world.Entities.Values.First(e=>e.Data.classname=="fps_portal");var destination=world.Entities[portal.Data.targets[0]];
            var portalBounds=portal.GetComponentsInChildren<Collider>().First(c=>c.isTrigger).bounds;
            player.Teleport(portalBounds.center-Vector3.up*.5f,Quaternion.identity,Vector3.zero);
            player.GetComponent<CharacterController>().Move(Vector3.up*.01f);
            for(int i=0;i<5;i++)yield return new WaitForFixedUpdate();
            var arrived=player.transform.position;
            Check(Vector3.Distance(arrived,destination.transform.position)<2,"Portal destination wrong");
            destination.Contact(player);Check(player.transform.position==arrived,"Portal bounced immediately");
            Debug.Log("HERON_GAMEPLAY portal PASS");
            player.Damage(35);Check(player.Health==65,"Player damage failed");player.Heal(20);Check(player.Health==85,"Healing failed");
            player.SetCheckpoint(new Vector3(100,2,100),Quaternion.identity);player.Damage(1000);Check(player.Health==0&&world.Paused,"Death failed");
            player.Respawn();Check(player.Health==100&&!world.Paused&&player.transform.position.x==100,"Respawn failed");
            var pickup=world.Entities.Values.First(e=>e.Data.classname=="fps_pickup");pickup.Contact(player);Check(world.Score>0&&!pickup.gameObject.activeSelf,"Pickup failed");
            var gate=world.Entities.Values.First(e=>e.Data.classname=="fps_gate");
            world.AddItem("present",3);gate.Activate(player);Check(world.Entities[gate.Data.targets[0]].Open,"Inventory gate failed");
            var pad=world.Entities.Values.First(e=>e.Data.classname=="fps_jump_pad");pad.Contact(player);
            Check(player.Velocity.y==pad.Data.Number("strength",17),"Jump pad failed");
            var spawner=world.Entities.Values.First(e=>e.Data.classname=="fps_spawner");spawner.Activate(player);
            yield return new WaitForSeconds(1.8f);
            Check(world.mapRoot.Cast<Transform>().Count(t=>t.name=="Spawned enemy")==3,"Spawner did not create enemies");
            Debug.Log("HERON_GAMEPLAY health_inventory_spawner PASS");
            var victim=world.Entities.Values.First(e=>e.Data.classname=="fps_enemy");victim.transform.position=new Vector3(100,1,105);
            player.Teleport(new Vector3(100,1,100),Quaternion.identity,Vector3.zero);Physics.SyncTransforms();
            float healthBeforeAttack=player.Health;victim.enabled=true;
            yield return new WaitForSeconds(1.1f);
            victim.enabled=false;
            Check(player.Health<healthBeforeAttack,"Enemy did not attack player");
            victim.transform.position=new Vector3(100,1,105);Physics.SyncTransforms();
            float hp=victim.Health;int ammo=player.Ammo;
            Check(player.Shoot(new Ray(new Vector3(100,1.8f,100),Vector3.forward)),"Shot rejected");
            Check(victim.Health==hp-25&&player.Ammo==ammo-1,"Hitscan did not damage enemy");
            Check(!player.Shoot(new Ray(player.View.transform.position,Vector3.forward)),"Fire rate not enforced");
            yield return new WaitForSeconds(.2f);
            victim.Damage(100);Check(!victim.gameObject.activeSelf,"Enemy death failed");
            Debug.Log("HERON_GAMEPLAY shooting PASS");
            var barrel=world.Entities.Values.First(e=>e.Data.classname=="fps_barrel");barrel.transform.position=player.transform.position+Vector3.forward;
            Physics.SyncTransforms();float before=player.Health;barrel.Damage(100);Check(player.Health<before&&!barrel.gameObject.activeSelf,"Explosion damage failed");
            world.Entities.Values.First(e=>e.Data.classname=="fps_exit").Contact(player);Check(world.Completed,"Exit did not complete level");
            Debug.Log("HERON_GAMEPLAY explosion_exit PASS");
        }
    }
}
