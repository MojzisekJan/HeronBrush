using System;
using System.Collections.Generic;
using System.Linq;
using UnityEngine;
namespace HeronBrush.Demo {
    public class HeronDemoWorld : MonoBehaviour {
        public TextAsset definition;
        public Transform mapRoot;
        public Transform liveAttachment;
        public bool captureMouse=true;
        public float moveSpeed=6, jumpSpeed=7, mouseSensitivity=.12f;
        public float maximumHealth=100, shotDamage=25, shotInterval=.18f;
        public int startingAmmo=60;
        public HeronDemoPlayer Player {get;private set;}
        public float UnitsPerMetre {get;private set;}=32;
        public readonly Dictionary<string,HeronDemoEntity> Entities=new Dictionary<string,HeronDemoEntity>();
        public readonly Dictionary<string,int> Inventory=new Dictionary<string,int>();
        public string Message {get;private set;}="WASD move · Space jump · E use · Mouse shoot";
        public bool Completed {get;private set;}
        public bool Paused {get;private set;}
        public int Score {get;private set;}
        public float Clock {get;private set;}
        void Update() {if(!Paused)Clock+=Time.deltaTime;}
        float messageUntil;
        void Start() { Initialize(); }
        public void Initialize() {
            if(Player) return;
            if(liveAttachment)mapRoot=liveAttachment.Find("Generated");
            if(!definition || !mapRoot) throw new InvalidOperationException("Assign exported gameplay data and map root.");
            var data=JsonUtility.FromJson<LevelData>(definition.text);
            if(data==null || data.version!=1 || data.entities==null) throw new InvalidOperationException("Unsupported Heron Brush gameplay data.");
            UnitsPerMetre=Mathf.Max(.001f,data.scale);
            foreach(var e in data.entities) {
                var node=mapRoot.Cast<Transform>().FirstOrDefault(t=>t.name==e.name);
                if(!node) {Debug.LogWarning("Missing demo entity: "+e.name,this);continue;}
                var behavior=node.gameObject.AddComponent<HeronDemoEntity>();
                behavior.Configure(this,e);Entities.Add(e.name,behavior);
            }
            var start=Entities.Values.FirstOrDefault(e=>e.Data.classname=="info_player_start");
            var playerObject=new GameObject("Demo player");
            playerObject.transform.SetParent(transform,false);
            playerObject.transform.position=start?start.transform.position:mapRoot.position+Vector3.up*2;
            playerObject.transform.rotation=start?start.transform.rotation*Quaternion.Euler(0,180,0):Quaternion.identity;
            Player=playerObject.AddComponent<HeronDemoPlayer>();Player.Configure(this);
            Show("WASD move · Space jump · E use · Mouse shoot",6);
            SetPaused(false);
        }
        public void ActivateTargets(EntityData data, HeronDemoPlayer player) {
            foreach(var name in data.targets ?? Array.Empty<string>()) if(Entities.TryGetValue(name,out var entity)) entity.Activate(player);
        }
        public void AddItem(string kind,int count) { Inventory[kind]=Count(kind)+count;Score+=count;Show("Collected "+kind); }
        public int Count(string kind)=>Inventory.TryGetValue(kind,out var n)?n:0;
        public void Show(string text,float seconds=3) { Message=text;messageUntil=Clock+seconds; }
        public void Finish(string text) {Completed=true;Show(text,3600);SetPaused(true);}
        public void SetPaused(bool paused) {
            Paused=paused;
            Cursor.lockState=captureMouse&&!paused?CursorLockMode.Locked:CursorLockMode.None;
            Cursor.visible=!captureMouse||paused;
        }
        void OnDestroy() {Cursor.lockState=CursorLockMode.None;Cursor.visible=true;}
        void OnGUI() {
            if(!Player) return;
            var style=new GUIStyle(GUI.skin.label){fontSize=22,normal={textColor=Color.white}};
            GUI.Box(new Rect(16,16,280,100),GUIContent.none);
            GUI.Label(new Rect(28,25,260,35),"Health  "+Mathf.CeilToInt(Player.Health)+" / "+maximumHealth,style);
            GUI.Label(new Rect(28,62,260,35),"Ammo  "+Player.Ammo+"    Score  "+Score,style);
            GUI.Label(new Rect(Screen.width/2-8,Screen.height/2-18,30,40),"+",style);
            if(Clock<messageUntil) GUI.Label(new Rect(25,Screen.height-60,Screen.width-50,50),Message,style);
            if(Player.Health<=0 || Completed || Paused) {
                var rect=new Rect(Screen.width/2-180,Screen.height/2-95,360,190);
                GUI.Box(rect,Completed?"Level complete":Player.Health<=0?"You died":"Paused");
                if(!Completed && GUI.Button(new Rect(rect.x+30,rect.y+50,300,45),Player.Health<=0?"Respawn":"Resume")) {
                    if(Player.Health<=0) Player.Respawn();SetPaused(false);
                }
                if(GUI.Button(new Rect(rect.x+30,rect.y+110,300,45),"Restart level"))
                    UnityEngine.SceneManagement.SceneManager.LoadScene(UnityEngine.SceneManagement.SceneManager.GetActiveScene().buildIndex);
            }
        }
    }
}
