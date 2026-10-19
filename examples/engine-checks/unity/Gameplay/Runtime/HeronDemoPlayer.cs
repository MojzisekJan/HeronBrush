using System.Linq;
using UnityEngine;
#if ENABLE_INPUT_SYSTEM
using UnityEngine.InputSystem;
#endif
namespace HeronBrush.Demo {
    [RequireComponent(typeof(CharacterController))]
    public class HeronDemoPlayer : MonoBehaviour {
        public float Health {get;private set;}=100;
        public int Ammo {get;private set;}=60;
        public Camera View {get;private set;}
        public Vector3 Velocity {get;private set;}
        public float PortalReadyAt;
        public HeronDemoWorld World {get;private set;}
        CharacterController controller;
        Vector3 checkpoint;
        Quaternion checkpointRotation;
        float pitch, shotReady;
        HeronDemoEntity platform;
        int platformTick=-1;
        HeronDemoEntity carriedPlatform;
        public void Configure(HeronDemoWorld world) {
            World=world;Health=world.maximumHealth;Ammo=world.startingAmmo;controller=GetComponent<CharacterController>();
            controller.height=1.8f;controller.radius=.3f;controller.center=Vector3.up*.9f;
            controller.stepOffset=.3f;controller.skinWidth=.03f;
            checkpoint=transform.position;checkpointRotation=transform.rotation;
            var cameraObject=new GameObject("Player camera");cameraObject.transform.SetParent(transform,false);
            cameraObject.transform.localPosition=Vector3.up*1.6f;
            View=cameraObject.AddComponent<Camera>();View.fieldOfView=75;View.nearClipPlane=.05f;
            cameraObject.AddComponent<AudioListener>();
            View.clearFlags=CameraClearFlags.SolidColor;View.backgroundColor=new Color(.18f,.25f,.35f);
        }
        void Update() {
            if(!World) return;
#if ENABLE_INPUT_SYSTEM
            var k=Keyboard.current; var m=Mouse.current;
            bool escape=k!=null&&k.escapeKey.wasPressedThisFrame;
            Vector2 move=k==null?Vector2.zero:new Vector2((k.dKey.isPressed?1:0)-(k.aKey.isPressed?1:0),(k.wKey.isPressed?1:0)-(k.sKey.isPressed?1:0));
            Vector2 look=m!=null?m.delta.ReadValue():Vector2.zero;
            bool jump=k!=null&&k.spaceKey.wasPressedThisFrame, use=k!=null&&k.eKey.wasPressedThisFrame, fire=m!=null&&m.leftButton.isPressed;
#else
            bool escape=Input.GetKeyDown(KeyCode.Escape);
            Vector2 move=new Vector2((Input.GetKey(KeyCode.D)?1:0)-(Input.GetKey(KeyCode.A)?1:0),(Input.GetKey(KeyCode.W)?1:0)-(Input.GetKey(KeyCode.S)?1:0));
            Vector2 look=new Vector2(Input.GetAxisRaw("Mouse X"),Input.GetAxisRaw("Mouse Y"))*10;
            bool jump=Input.GetKeyDown(KeyCode.Space),use=Input.GetKeyDown(KeyCode.E),fire=Input.GetMouseButton(0);
#endif
            if(escape && Health>0 && !World.Completed) World.SetPaused(!World.Paused);
            if(World.Paused || Health<=0) return;
            if(Cursor.lockState==CursorLockMode.Locked) {
                transform.Rotate(0,look.x*World.mouseSensitivity,0);
                pitch=Mathf.Clamp(pitch-look.y*World.mouseSensitivity,-85,85);
                View.transform.localRotation=Quaternion.Euler(pitch,0,0);
            }
            StepMovement(move,jump,Time.deltaTime);
            if(use) Use();
            if(fire) Shoot(new Ray(View.transform.position,View.transform.forward));
            if(transform.position.y<-50) Damage(1000);
        }
        public void StepMovement(Vector2 input,bool jump,float dt) {
            if(World.Paused || Health<=0) return;
            var horizontal=transform.TransformDirection(new Vector3(input.x,0,input.y).normalized)*World.moveSpeed;
            float vertical=controller.isGrounded&&Velocity.y<0?-2:Velocity.y;
            if(jump && controller.isGrounded) vertical=World.jumpSpeed;
            Velocity=horizontal+Vector3.up*(vertical-20*dt);
            if(platform && controller.isGrounded && (platform!=carriedPlatform || platformTick!=platform.PhysicsTick)) {
                controller.Move(platform.FrameDelta);platformTick=platform.PhysicsTick;carriedPlatform=platform;
            }
            platform=null;controller.Move(Velocity*dt);
        }
        void OnControllerColliderHit(ControllerColliderHit hit) {
            if(hit.normal.y>.5f) platform=hit.collider.GetComponentInParent<HeronDemoEntity>();
        }
        public bool Shoot(Ray ray) {
            if(World.Paused || Health<=0 || Ammo<=0 || World.Clock<shotReady) return false;
            shotReady=World.Clock+Mathf.Max(.03f,World.shotInterval);Ammo--;
            Vector3 end=ray.GetPoint(60);
            if(Physics.Raycast(ray,out var hit,60,~0,QueryTriggerInteraction.Ignore)) {
                end=hit.point;
                var victim=hit.collider.GetComponentInParent<HeronDemoEntity>();
                if(victim) victim.Damage(World.shotDamage);
            }
            var trail=new GameObject("Shot");var line=trail.AddComponent<LineRenderer>();
            line.positionCount=2;line.SetPosition(0,ray.origin+ray.direction*.3f);line.SetPosition(1,end);
            line.startWidth=.025f;line.endWidth=.005f;
            var material=new Material(Shader.Find("Sprites/Default"));material.color=new Color(1,.8f,.2f);line.sharedMaterial=material;
            Destroy(material,.09f);Destroy(trail,.09f);
            return true;
        }
        public void Use() {
            if(Physics.Raycast(View.transform.position,View.transform.forward,out var hit,3,~0,QueryTriggerInteraction.Ignore))
                hit.collider.GetComponentInParent<HeronDemoEntity>()?.Activate(this);
        }
        public void Damage(float amount) {if(Health<=0)return;Health=Mathf.Max(0,Health-Mathf.Max(0,amount));if(Health<=0){World.Show("You died",3600);World.SetPaused(true);}}
        public void Heal(float amount) {Health=Mathf.Min(World.maximumHealth,Health+Mathf.Max(0,amount));}
        public void AddAmmo(int amount) {Ammo=Mathf.Max(0,Ammo+amount);}
        public void SetCheckpoint(Vector3 position,Quaternion rotation) {checkpoint=position;checkpointRotation=rotation;World.Show("Checkpoint reached");}
        public void Respawn() {Health=World.maximumHealth;Ammo=World.startingAmmo;Teleport(checkpoint,checkpointRotation,Vector3.zero);World.SetPaused(false);}
        public void Launch(float speed) {Velocity=new Vector3(Velocity.x,speed,Velocity.z);}
        public void Teleport(Vector3 position,Quaternion rotation,Vector3 velocity) {
            controller.enabled=false;transform.SetPositionAndRotation(position,rotation);controller.enabled=true;
            Velocity=velocity;pitch=0;View.transform.localRotation=Quaternion.identity;platform=null;
        }
    }
}
