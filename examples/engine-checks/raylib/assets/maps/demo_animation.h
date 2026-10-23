#ifndef HERON_EXPORTED_ANIMATION_H
#define HERON_EXPORTED_ANIMATION_H
/* Generated animation runtime: C99, no JSON parser or raylib dependency. */
#include <math.h>
#include <string.h>
typedef struct { double time,value,incoming,outgoing; int interpolation; } HbKey;
typedef struct { const HbKey *keys; int count; } HbChannel;
typedef struct { const char *name; double duration; int mode; HbChannel channels[6]; } HbClip;
typedef struct { const char *name; int entity,first,count,selected,autoplay; double pivot[3],scale; } HbAnimation;
typedef struct { int clip,playing,posed; double time; } HbPlayer;
#define HB_ANIMATION_COUNT 0
static const HbClip hb_clips[]={
{0}
};
static const HbAnimation hb_animations[]={
{0}
};
/* All time arguments are seconds. mode: 0 once, 1 loop, 2 ping-pong. */
static inline double HbSample(HbChannel c,double time) {
    if (!c.count) return 0;
    if (time<=c.keys[0].time) return c.keys[0].value;
    int lo=0,hi=c.count;
    while(lo<hi){int mid=lo+(hi-lo)/2;if(c.keys[mid].time<=time)lo=mid+1;else hi=mid;}
    const HbKey *a=&c.keys[lo-1];if(lo==c.count)return a->value;
    const HbKey *b=&c.keys[lo];double dt=b->time-a->time,t=(time-a->time)/dt,u=1-t;
    if(a->interpolation==1)return a->value;
    if(a->interpolation==2)return u*u*u*a->value+3*u*u*t*(a->value+a->outgoing*dt/3)+3*u*t*t*(b->value-b->incoming*dt/3)+t*t*t*b->value;
    return a->value+(b->value-a->value)*t;
}
static inline double HbTime(const HbClip *c,double t) {
    if(!isfinite(t)||t<0)t=0;
    if(c->mode==0)return fmin(t,c->duration);
    t=fmod(t,c->duration*(c->mode==2?2:1));
    return t>c->duration?2*c->duration-t:t;
}
static inline int HbPlay(HbPlayer *p,int animation,const char *name) {
    if(animation<0||animation>=HB_ANIMATION_COUNT)return 0;
    const HbAnimation *a=&hb_animations[animation];
    int clip=a->selected;
    if(name){clip=-1;for(int i=a->first;i<a->first+a->count;i++)if(!strcmp(hb_clips[i].name,name)){clip=i;break;}}
    if(clip<0)return 0;
    *p=(HbPlayer){clip,1,1,0};return 1;
}
static inline void HbInit(HbPlayer *p,int animation) {
    *p=(HbPlayer){0,0,0,0};
    if(animation<0||animation>=HB_ANIMATION_COUNT)return;
    p->clip=hb_animations[animation].selected;
    if(hb_animations[animation].autoplay)HbPlay(p,animation,0);
}
static inline void HbUpdate(HbPlayer *p,double dt) {
    if(!p->playing||!isfinite(dt)||dt<0)return;
    const HbClip *c=&hb_clips[p->clip];p->time+=dt;
    if(c->mode==0&&p->time>=c->duration){p->time=c->duration;p->playing=0;}
    else if(c->mode) p->time=fmod(p->time,c->duration*(c->mode==2?2:1));
}
static inline void HbSeek(HbPlayer *p,double t) {if(isfinite(t)){p->time=fmax(0,t);p->posed=1;}}
static inline void HbPause(HbPlayer *p){p->playing=0;}
static inline void HbResume(HbPlayer *p){p->playing=1;p->posed=1;}
static inline void HbStop(HbPlayer *p){p->playing=0;p->posed=0;p->time=0;}
/* Input/output: WORLD engine coordinates, (map Y,Z,X)/map units per metre.
   Use for collision vertices as well as rendering. normal=1 omits translation. */
static inline void HbPoint(int animation,const HbPlayer *p,const double in[3],double out[3],int normal) {
    if(animation<0||animation>=HB_ANIMATION_COUNT||!p->posed){memcpy(out,in,3*sizeof(double));return;}
    const HbAnimation *a=&hb_animations[animation];const HbClip *c=&hb_clips[p->clip];
    double t=HbTime(c,p->time),v[3]={in[2],in[0],in[1]};
    if(!normal)for(int i=0;i<3;i++)v[i]-=a->pivot[i]/a->scale;
    for(int axis=0;axis<3;axis++){
        double r=HbSample(c->channels[axis+3],t)*0.017453292519943295,s=sin(r),co=cos(r);
        int j=(axis+1)%3,k=(axis+2)%3;double x=v[j],y=v[k];v[j]=co*x-s*y;v[k]=s*x+co*y;
    }
    if(!normal)for(int i=0;i<3;i++)v[i]+=(a->pivot[i]+HbSample(c->channels[i],t))/a->scale;
    out[0]=v[1];out[1]=v[2];out[2]=v[0];
}
/* Column-major affine transform to apply AFTER the entity's rest transform. */
static inline void HbMatrix(int animation,const HbPlayer *p,double m[16]) {
    memset(m,0,16*sizeof(double));m[15]=1;
    for(int i=0;i<4;i++){double v[3]={0},out[3];if(i<3)v[i]=1;
        HbPoint(animation,p,v,out,i<3);for(int j=0;j<3;j++)m[i*4+j]=out[j];}
}
typedef struct {const double (*planes)[4];int count;} HbVolume;
typedef struct {const char *name;const HbVolume *volumes;int count;const int *targets;int target_count,once;double wait;const char *action,*clip;} HbTrigger;
#define HB_TRIGGER_COUNT 0
static const HbTrigger hb_triggers[]={{0}};
typedef struct {int inside,fired;double next;} HbTriggerState;
static inline void HbActivateAnimation(HbPlayer *p,int animation) {
    const HbAnimation *a=&hb_animations[animation];int open=-1,close=-1;
    for(int i=a->first;i<a->first+a->count;i++){if(!strcmp(hb_clips[i].name,"open"))open=i;if(!strcmp(hb_clips[i].name,"close"))close=i;}
    if(open>=0&&close>=0){int next=p->posed&&p->clip==open?close:open;double phase=fmin(1,p->time/hb_clips[p->clip].duration);int reverse=p->posed&&p->playing;HbPlay(p,animation,hb_clips[next].name);if(reverse)HbSeek(p,(1-phase)*hb_clips[next].duration);}
    else HbPlay(p,animation,0);
}
static inline int HbAnimationAction(HbPlayer *p,int id,const char *action,const char *clip) {
    if(!action)action="toggle";
    if(!strcmp(action,"play"))return HbPlay(p,id,clip&&clip[0]?clip:0);
    if(!strcmp(action,"open")||!strcmp(action,"close"))return HbPlay(p,id,action);
    if(!strcmp(action,"pause"))HbPause(p);
    else if(!strcmp(action,"resume"))HbResume(p);
    else if(!strcmp(action,"stop"))HbStop(p);
    else if(!strcmp(action,"toggle")||!action[0])HbActivateAnimation(p,id);
    else return 0;
    return 1;
}
/* Call explicitly for a button, or use HbCheckTriggers for a player point and
   HbCheckTriggersSpan for a standing player. */
static inline int HbActivateTrigger(HbTriggerState *states,HbPlayer *players,int trigger,double now) {
    if(trigger<0||trigger>=HB_TRIGGER_COUNT||!isfinite(now))return 0;
    const HbTrigger *t=&hb_triggers[trigger];HbTriggerState *s=&states[trigger];
    if((t->once&&s->fired)||now<s->next)return 0;
    int delivered=0;
    for(int i=0;i<t->target_count;i++)delivered|=HbAnimationAction(&players[t->targets[i]],t->targets[i],t->action,t->clip);
    if(delivered){s->fired=1;s->next=now+t->wait;}
    return delivered;
}
static inline int HbTriggerContains(int trigger,const double point[3]) {
    const HbTrigger *t=&hb_triggers[trigger];
    for(int v=0;v<t->count;v++){int fits=1;HbVolume volume=t->volumes[v];for(int p=0;p<volume.count;p++){const double *q=volume.planes[p];if(q[0]*point[0]+q[1]*point[1]+q[2]*point[2]>q[3]+1e-5){fits=0;break;}}if(fits)return 1;}
    return 0;
}
static inline void HbCheckTriggers(HbTriggerState *states,HbPlayer *players,const double point[3],double now) {
    for(int i=0;i<HB_TRIGGER_COUNT;i++){
        int inside=HbTriggerContains(i,point);
        if(inside&&!states[i].inside)HbActivateTrigger(states,players,i,now);
        states[i].inside=inside;
    }
}
/* A standing player: its feet, middle and head, `height` metres up from `feet`. A
   volume that starts above the floor, as a raised trigger brush does, still catches it. */
static inline void HbCheckTriggersSpan(HbTriggerState *states,HbPlayer *players,const double feet[3],double height,double now) {
    for(int i=0;i<HB_TRIGGER_COUNT;i++){
        int inside=0;
        for(int k=0;k<3&&!inside;k++){double p[3]={feet[0],feet[1]+height*k/2,feet[2]};inside=HbTriggerContains(i,p);}
        if(inside&&!states[i].inside)HbActivateTrigger(states,players,i,now);
        states[i].inside=inside;
    }
}

#endif
