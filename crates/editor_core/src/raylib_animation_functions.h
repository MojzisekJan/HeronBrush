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
