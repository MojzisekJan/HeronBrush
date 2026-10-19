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
/* Call explicitly for a button, or use HbCheckTriggers for a player point. */
static inline int HbActivateTrigger(HbTriggerState *states,HbPlayer *players,int trigger,double now) {
    if(trigger<0||trigger>=HB_TRIGGER_COUNT||!isfinite(now))return 0;
    const HbTrigger *t=&hb_triggers[trigger];HbTriggerState *s=&states[trigger];
    if((t->once&&s->fired)||now<s->next)return 0;
    int delivered=0;
    for(int i=0;i<t->target_count;i++)delivered|=HbAnimationAction(&players[t->targets[i]],t->targets[i],t->action,t->clip);
    if(delivered){s->fired=1;s->next=now+t->wait;}
    return delivered;
}
static inline void HbCheckTriggers(HbTriggerState *states,HbPlayer *players,const double point[3],double now) {
    for(int i=0;i<HB_TRIGGER_COUNT;i++){
        int inside=0;const HbTrigger *t=&hb_triggers[i];
        for(int v=0;v<t->count;v++){int fits=1;HbVolume volume=t->volumes[v];for(int p=0;p<volume.count;p++){const double *q=volume.planes[p];if(q[0]*point[0]+q[1]*point[1]+q[2]*point[2]>q[3]+1e-5){fits=0;break;}}if(fits){inside=1;break;}}
        if(inside&&!states[i].inside)HbActivateTrigger(states,players,i,now);
        states[i].inside=inside;
    }
}
